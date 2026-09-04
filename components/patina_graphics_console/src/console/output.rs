//! Produces EFI Simple Text Output Protocol over a single Graphics Output controller.
//!
//! [`SimpleTextOutputHolder`] is the per-controller console that owns the controller's opened GOP
//! interface, the located HII Font interface, the computed text-mode list, and the public
//! EFI Simple Text Output Protocol/`Mode` structures a caller (such as BDS, con splitter, or an
//! application) reads and calls into directly.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use alloc::{boxed::Box, vec::Vec};

use patina::{
    BinaryGuid,
    component::service::{Service, uefi_services::tpl::TplServices},
    error::{EfiError, Result},
    protocol::ProtocolInterface,
    standard::efi::{self, protocols::simple_text_output},
    string::Char16Str,
    uefi::{boot_services::tpl::Tpl, tpl_mutex::TplMutex},
};

use super::{
    font::HiiFontHandle,
    gop::{BLACK_PIXEL, BltPixel, GLYPH_HEIGHT, GLYPH_WIDTH, GopHandle, TEXT_COLORS},
    mode::{TextMode, build_text_modes, pick_preferred_mode},
};

/// UEFI Spec "BS" definition.
/// Moves the cursor left one column. If the cursor is at the left margin, no action is taken.
const CHAR_BACKSPACE: u16 = 0x0008;
/// UEFI Spec "LF" definition.
/// Moves the cursor to the next line.
const CHAR_LINEFEED: u16 = 0x000A;
/// UEFI Spec "CR" definition.
/// Moves the cursor to the left margin (beginning) of the current line.
const CHAR_CARRIAGE_RETURN: u16 = 0x000D;
/// `NARROW_CHAR` - Toggles off double-width rendering for the characters that follow, until
/// the next `WIDE_CHAR`.
const NARROW_CHAR: u16 = 0xFFF0;
/// `WIDE_CHAR` - Toggles on double-width rendering for the characters that follow, until the
/// next `NARROW_CHAR`.
const WIDE_CHAR: u16 = 0xFFF1;
/// `EFI_WIDE_ATTRIBUTE` - The attribute bit that indicates the characters that follow should
/// be rendered double-width.
const WIDE_ATTRIBUTE: i32 = 0x80;
/// `EFI_TEXT_ATTR(EFI_LIGHTGRAY, EFI_BLACK)` - Light gray text on a black background.
const DEFAULT_ATTRIBUTE: i32 = 0x07;
/// A blank narrow-glyph cell, used to erase the character behind a backspace or line-wrap pad.
const SPACE: u16 = b' ' as u16;

/// The narrow "block" cursor bitmap: a solid bar across the bottom 3 rows of the 8x19 glyph cell.
const CURSOR_GLYPH: [u8; GLYPH_HEIGHT] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF];

/// Private, per-controller console state, guarded by a `TplMutex` at `TPL_NOTIFY` so a render or
/// mode change cannot interleave with another one.
struct ConsoleState {
    gop: GopHandle,
    hii_font: HiiFontHandle,
    modes: Vec<TextMode>,
}

/// Returns a copy of the text mode at `index`, so callers never index `modes` directly.
fn text_mode_at(modes: &[TextMode], index: usize) -> Result<TextMode> {
    modes.get(index).copied().ok_or(EfiError::Unsupported)
}
/// Wraps EFI Simple Text Output Protocol alongside the console's private state.
///
/// `protocol` is the first field so a pointer to this struct can be safely reinterpreted as a
/// pointer to `simple_text_output::Protocol`.
#[repr(C)]
pub(crate) struct SimpleTextOutputHolder {
    protocol: simple_text_output::Protocol,
    mode: Box<simple_text_output::Mode>,
    state: TplMutex<ConsoleState, Service<dyn TplServices>>,
}

// SAFETY: `simple_text_output::Protocol` has a `ProtocolInterface` confirming its layout matches
// the UEFI Simple Text Output protocol GUID. `SimpleTextOutputHolder` is `#[repr(C)]` with `protocol`
// as its first field, so a pointer to a `SimpleTextOutputHolder` is also a valid pointer to a
// `simple_text_output::Protocol`.
unsafe impl ProtocolInterface for SimpleTextOutputHolder {
    const PROTOCOL_GUID: BinaryGuid = BinaryGuid(simple_text_output::PROTOCOL_GUID);
}

impl SimpleTextOutputHolder {
    /// Builds a new, ready-to-install console for one GOP controller.
    ///
    /// `resolution`/`text_mode` are the platform's preferred pixel resolution and text-mode
    /// dimensions (see [`super::pcd::ConsolePreferences`]). `None` means "pick the largest
    /// available".
    pub(crate) fn new(
        mut gop: GopHandle,
        hii_font: HiiFontHandle,
        tpl: Service<dyn TplServices>,
        resolution: Option<(u32, u32)>,
        text_mode: Option<(u32, u32)>,
    ) -> Result<Box<Self>> {
        let (gop_mode_number, horizontal, vertical) = super::mode::select_gop_mode(&mut gop, resolution)?;
        let modes = build_text_modes(horizontal, vertical, gop_mode_number);
        let preferred = pick_preferred_mode(&modes, text_mode);
        let max_mode = i32::try_from(modes.len()).map_err(|_| EfiError::InvalidParameter)?;

        let mode = Box::new(simple_text_output::Mode {
            max_mode,
            mode: -1,
            attribute: DEFAULT_ATTRIBUTE,
            cursor_column: 0,
            cursor_row: 0,
            cursor_visible: efi::Boolean::FALSE,
        });
        // The `Box`'s heap allocation does not move again, so this pointer stays valid for the
        // lifetime of `mode` (which lives exactly as long as the enclosing `Self`).
        let mode_ptr: *mut simple_text_output::Mode = core::ptr::from_ref(mode.as_ref()).cast_mut();

        let protocol = simple_text_output::Protocol {
            reset: trampoline::reset,
            output_string: trampoline::output_string,
            test_string: trampoline::test_string,
            query_mode: trampoline::query_mode,
            set_mode: trampoline::set_mode,
            set_attribute: trampoline::set_attribute,
            clear_screen: trampoline::clear_screen,
            set_cursor_position: trampoline::set_cursor_position,
            enable_cursor: trampoline::enable_cursor,
            mode: mode_ptr,
        };

        let state = TplMutex::new(tpl, Tpl::NOTIFY, ConsoleState { gop, hii_font, modes });
        let holder = Box::new(Self { protocol, mode, state });
        holder.set_mode(preferred)?;
        Ok(holder)
    }

    /// Returns a writable pointer to the public `Mode` struct.
    ///
    /// Obtaining the pointer is safe, but writing through it is only sound while holding a
    /// [`TplMutex`] guard from `self.state`, which raises the task priority level to serialize
    /// access.
    fn mode_ptr(&self) -> *mut simple_text_output::Mode {
        core::ptr::from_ref(self.mode.as_ref()).cast_mut()
    }

    /// Resets the device by setting mode 0, then the default foreground color on a black background.
    pub(crate) fn reset(&self) -> Result<()> {
        self.set_mode(0)?;
        // SAFETY: There is no concurrent access. `set_mode` above already released its own guard, and
        // `attribute` is only ever written while a guard is held, so a momentary unguarded read of
        // its current value here can occur.
        let attribute = unsafe { (*self.mode_ptr()).attribute };
        self.set_attribute((attribute & 0x0F) as usize)
    }

    /// Writes `text` to the console at the current cursor position.
    ///
    /// Returns `Ok(true)` if every character had a glyph, or `Ok(false)` if one or more characters
    /// had none and were skipped, matching `EFI_WARN_UNKNOWN_GLYPH`.
    ///
    /// The cursor block is only erased once at the start and redrawn once at the end (rather than
    /// after every intermediate move), since this method tracks the cursor position in a local variable
    /// and only commits it to the public `Mode` struct.
    pub(crate) fn output_string(&self, text: &Char16Str) -> Result<bool> {
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();

        // SAFETY: guarded by `guard`, held for the remainder of this function.
        let mode_index = unsafe { (*mode_ptr).mode };
        if mode_index < 0 {
            return Err(EfiError::Unsupported);
        }
        let text_mode = text_mode_at(&guard.modes, mode_index as usize)?;
        let max_column = text_mode.columns;
        let max_row = text_mode.rows;

        self.flush_cursor(&mut guard, mode_ptr)?;

        // SAFETY: guarded by `guard`.
        let original_attribute = unsafe { (*mode_ptr).attribute };
        let mut attribute = original_attribute;
        // SAFETY: guarded by `guard`.
        let (mut column, mut row) = unsafe { ((*mode_ptr).cursor_column as usize, (*mode_ptr).cursor_row as usize) };
        let mut all_known = true;

        let units = text.as_units();
        let mut index = 0usize;
        while index < units.len() {
            let Some(&unit) = units.get(index) else { break };
            match unit {
                CHAR_BACKSPACE => {
                    if column == 0 && row > 0 {
                        row -= 1;
                        column = max_column - 1;
                    } else if column > 0 {
                        column -= 1;
                    } else {
                        index += 1;
                        continue;
                    }
                    let (foreground, background) = text_colors(attribute);
                    self.draw_run(&mut guard, &text_mode, (column, row), &[SPACE], foreground, background)?;
                    index += 1;
                }
                CHAR_LINEFEED => {
                    if row == max_row - 1 {
                        self.scroll_up(&mut guard, &text_mode)?;
                    } else {
                        row += 1;
                    }
                    index += 1;
                }
                CHAR_CARRIAGE_RETURN => {
                    column = 0;
                    index += 1;
                }
                WIDE_CHAR => {
                    attribute |= WIDE_ATTRIBUTE;
                    index += 1;
                }
                NARROW_CHAR => {
                    attribute &= !WIDE_ATTRIBUTE;
                    index += 1;
                }
                _ => {
                    let wide = (attribute & WIDE_ATTRIBUTE) != 0;
                    let (count, width) = run_length(units, index, column, max_column, wide);
                    if count == 0 {
                        // Nothing more fits on this line and it isn't a control character, so stop
                        // rather than loop forever. Should not occur in practice, since the wrap
                        // logic below always leaves `column` within bounds before the next
                        // iteration reaches this branch again.
                        break;
                    }

                    let (foreground, background) = text_colors(attribute);
                    let ok = self.draw_run(
                        &mut guard,
                        &text_mode,
                        (column, row),
                        units.get(index..index + count).ok_or(EfiError::Unsupported)?,
                        foreground,
                        background,
                    )?;
                    all_known &= ok;
                    index += count;
                    column += width;

                    if column > max_column {
                        column -= 2;
                        self.draw_run(&mut guard, &text_mode, (column, row), &[SPACE], foreground, background)?;
                    }

                    if column >= max_column {
                        column = 0;
                        if row == max_row - 1 {
                            self.scroll_up(&mut guard, &text_mode)?;
                        } else {
                            row += 1;
                        }
                    }
                }
            }
        }

        // SAFETY: guarded by `guard`.
        unsafe {
            (*mode_ptr).attribute = original_attribute;
            (*mode_ptr).cursor_column = column as i32;
            (*mode_ptr).cursor_row = row as i32;
        }
        self.flush_cursor(&mut guard, mode_ptr)?;
        Ok(all_known)
    }

    /// Returns whether every character in `text` has a glyph in the system default font.
    pub(crate) fn test_string(&self, text: &Char16Str) -> bool {
        let guard = self.state.lock();
        for &unit in text.as_units() {
            if unit == WIDE_CHAR || unit == NARROW_CHAR {
                // A plain iterator always advances to prevent a potential hang.
                continue;
            }
            if !guard.hii_font.has_glyph(unit) {
                return false;
            }
        }
        true
    }

    /// Returns the columns/rows of `mode_number`.
    pub(crate) fn query_mode(&self, mode_number: usize) -> Result<(usize, usize)> {
        let guard = self.state.lock();
        // SAFETY: guarded by `guard`.
        if mode_number >= unsafe { (*self.mode_ptr()).max_mode } as usize {
            return Err(EfiError::Unsupported);
        }
        let text_mode = text_mode_at(&guard.modes, mode_number)?;
        if text_mode.columns == 0 || text_mode.rows == 0 {
            return Err(EfiError::Unsupported);
        }
        Ok((text_mode.columns, text_mode.rows))
    }

    /// Sets the active text mode to `mode_number`.
    ///
    /// Calling this with the mode already active clears the screen instead of a no-op.
    pub(crate) fn set_mode(&self, mode_number: usize) -> Result<()> {
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();
        // SAFETY: guarded by `guard`.
        let (max_mode, current_mode) = unsafe { ((*mode_ptr).max_mode, (*mode_ptr).mode) };
        if mode_number >= max_mode as usize {
            return Err(EfiError::Unsupported);
        }

        let text_mode = text_mode_at(&guard.modes, mode_number)?;
        if text_mode.columns == 0 || text_mode.rows == 0 {
            return Err(EfiError::Unsupported);
        }

        if current_mode >= 0 && current_mode as usize == mode_number {
            drop(guard);
            return self.clear_screen();
        }

        if current_mode >= 0 {
            self.flush_cursor(&mut guard, mode_ptr)?;
        }

        if text_mode.gop_mode_number == guard.gop.current_mode_number() {
            guard.gop.blt_fill(BLACK_PIXEL, 0, 0, text_mode.gop_width as usize, text_mode.gop_height as usize)?;
        } else {
            guard.gop.set_mode(text_mode.gop_mode_number)?;
        }

        // SAFETY: guarded by `guard`.
        unsafe {
            (*mode_ptr).mode = mode_number as i32;
            (*mode_ptr).cursor_column = 0;
            (*mode_ptr).cursor_row = 0;
        }
        self.flush_cursor(&mut guard, mode_ptr)
    }

    /// Sets the foreground/background color attribute for `OutputString()`/`ClearScreen()`.
    pub(crate) fn set_attribute(&self, attribute: usize) -> Result<()> {
        if (attribute | 0x7F) != 0x7F {
            return Err(EfiError::Unsupported);
        }
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();
        // SAFETY: guarded by `guard`.
        if unsafe { (*mode_ptr).attribute } == attribute as i32 {
            return Ok(());
        }
        self.flush_cursor(&mut guard, mode_ptr)?;
        // SAFETY: guarded by `guard`.
        unsafe { (*mode_ptr).attribute = attribute as i32 };
        self.flush_cursor(&mut guard, mode_ptr)
    }

    /// Clears the display to the current background color and homes the cursor.
    pub(crate) fn clear_screen(&self) -> Result<()> {
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();
        // SAFETY: guarded by `guard`.
        let (mode_index, attribute) = unsafe { ((*mode_ptr).mode, (*mode_ptr).attribute) };
        if mode_index < 0 {
            return Err(EfiError::Unsupported);
        }
        let text_mode = text_mode_at(&guard.modes, mode_index as usize)?;
        let (_, background) = text_colors(attribute);

        guard.gop.blt_fill(background, 0, 0, text_mode.gop_width as usize, text_mode.gop_height as usize)?;
        // SAFETY: guarded by `guard`.
        unsafe {
            (*mode_ptr).cursor_column = 0;
            (*mode_ptr).cursor_row = 0;
        }
        self.flush_cursor(&mut guard, mode_ptr)
    }

    /// Moves the cursor to (`column`, `row`).
    pub(crate) fn set_cursor_position(&self, column: usize, row: usize) -> Result<()> {
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();
        // SAFETY: guarded by `guard`.
        let mode_index = unsafe { (*mode_ptr).mode };
        if mode_index < 0 {
            return Err(EfiError::Unsupported);
        }
        let text_mode = text_mode_at(&guard.modes, mode_index as usize)?;
        if column >= text_mode.columns || row >= text_mode.rows {
            return Err(EfiError::Unsupported);
        }
        // SAFETY: guarded by `guard`.
        if unsafe { (*mode_ptr).cursor_column == column as i32 && (*mode_ptr).cursor_row == row as i32 } {
            return Ok(());
        }
        self.flush_cursor(&mut guard, mode_ptr)?;
        // SAFETY: guarded by `guard`.
        unsafe {
            (*mode_ptr).cursor_column = column as i32;
            (*mode_ptr).cursor_row = row as i32;
        }
        self.flush_cursor(&mut guard, mode_ptr)
    }

    /// Shows or hides the cursor.
    pub(crate) fn enable_cursor(&self, visible: bool) -> Result<()> {
        let mut guard = self.state.lock();
        let mode_ptr = self.mode_ptr();
        // SAFETY: guarded by `guard`.
        if unsafe { (*mode_ptr).mode } < 0 {
            return Err(EfiError::Unsupported);
        }
        self.flush_cursor(&mut guard, mode_ptr)?;
        // SAFETY: guarded by `guard`.
        unsafe { (*mode_ptr).cursor_visible = efi::Boolean::from(visible) };
        self.flush_cursor(&mut guard, mode_ptr)
    }

    /// Draws one run of same-width-attribute characters at `position` (column, row), returning
    /// whether every character had a glyph.
    fn draw_run(
        &self,
        guard: &mut ConsoleState,
        text_mode: &TextMode,
        position: (usize, usize),
        units: &[u16],
        foreground: BltPixel,
        background: BltPixel,
    ) -> Result<bool> {
        // `StringToImage` requires a NUL-terminated string. `units` is a slice into the caller's
        // buffer with no terminator of its own, so it is copied into a small owned, NUL-terminated
        // buffer first.
        let mut nul_terminated = Vec::with_capacity(units.len() + 1);
        nul_terminated.extend_from_slice(units);
        nul_terminated.push(0);

        let (column, row) = position;
        let x = column * GLYPH_WIDTH + text_mode.delta_x;
        let y = row * GLYPH_HEIGHT + text_mode.delta_y;
        guard.hii_font.draw_to_screen(super::font::DrawToScreenRequest {
            screen: guard.gop.interface(),
            screen_width: text_mode.gop_width as u16,
            screen_height: text_mode.gop_height as u16,
            text: &nul_terminated,
            x,
            y,
            foreground,
            background,
        })
    }

    /// Scrolls the text area up by one glyph row, blanking the newly revealed last row.
    fn scroll_up(&self, guard: &mut ConsoleState, text_mode: &TextMode) -> Result<()> {
        // SAFETY: guarded by the caller holding `self.state`'s lock (the guard reference itself).
        let attribute = unsafe { (*self.mode_ptr()).attribute };
        let (_, background) = text_colors(attribute);

        let width = text_mode.columns * GLYPH_WIDTH;
        let height = (text_mode.rows - 1) * GLYPH_HEIGHT;
        guard.gop.blt_video_to_video(
            text_mode.delta_x,
            text_mode.delta_y + GLYPH_HEIGHT,
            text_mode.delta_x,
            text_mode.delta_y,
            width,
            height,
        )?;
        guard.gop.blt_fill(background, text_mode.delta_x, text_mode.delta_y + height, width, GLYPH_HEIGHT)
    }

    /// Toggles the cursor's visual representation on screen by XOR-ing the block cursor bitmap
    /// into the frame buffer. Calling this twice in a row (erase, then redraw) leaves the display
    /// unchanged, since XOR is self-inverse.
    fn flush_cursor(&self, guard: &mut ConsoleState, mode_ptr: *mut simple_text_output::Mode) -> Result<()> {
        // SAFETY: guarded by the caller holding `self.state`'s lock.
        let (visible, column, row, mode_index, attribute) = unsafe {
            (
                (*mode_ptr).cursor_visible,
                (*mode_ptr).cursor_column,
                (*mode_ptr).cursor_row,
                (*mode_ptr).mode,
                (*mode_ptr).attribute,
            )
        };
        if !bool::from(visible) || mode_index < 0 {
            return Ok(());
        }

        let text_mode = text_mode_at(&guard.modes, mode_index as usize)?;
        let glyph_x = column as usize * GLYPH_WIDTH + text_mode.delta_x;
        let glyph_y = row as usize * GLYPH_HEIGHT + text_mode.delta_y;

        let mut cell = [BltPixel { blue: 0, green: 0, red: 0, reserved: 0 }; GLYPH_WIDTH * GLYPH_HEIGHT];
        guard.gop.blt_video_to_buffer(&mut cell, glyph_x, glyph_y, GLYPH_WIDTH, GLYPH_HEIGHT)?;

        let (foreground, _) = text_colors(attribute);
        for (glyph_row, glyph_bits) in CURSOR_GLYPH.iter().enumerate() {
            for glyph_column in 0..GLYPH_WIDTH {
                if glyph_bits & (1 << glyph_column) == 0 {
                    continue;
                }
                // Note: Bit 0 of the glyph row maps to the rightmost pixel column, not the leftmost.
                let Some(pixel) = cell.get_mut(glyph_row * GLYPH_WIDTH + (GLYPH_WIDTH - glyph_column - 1)) else {
                    continue;
                };
                pixel.blue ^= foreground.blue;
                pixel.green ^= foreground.green;
                pixel.red ^= foreground.red;
                pixel.reserved ^= foreground.reserved;
            }
        }

        guard.gop.blt_buffer_to_video(&cell, glyph_x, glyph_y, GLYPH_WIDTH, GLYPH_HEIGHT)
    }
}

/// Maps an attribute byte to its foreground/background colors.
fn text_colors(attribute: i32) -> (BltPixel, BltPixel) {
    let attribute = attribute & 0x7F;
    let foreground = TEXT_COLORS.get((attribute & 0x0F) as usize).copied().unwrap_or(BLACK_PIXEL);
    let background = TEXT_COLORS.get((attribute >> 4) as usize).copied().unwrap_or(BLACK_PIXEL);
    (foreground, background)
}

/// Determines how many `units` starting at `index` can be drawn in one run before hitting a
/// control character or the end of the line, and how many display columns they occupy (double for
/// wide characters).
fn run_length(units: &[u16], index: usize, cursor_column: usize, max_column: usize, wide: bool) -> (usize, usize) {
    let mut count = 0usize;
    let mut width = 0usize;
    while cursor_column + width < max_column {
        let Some(&unit) = units.get(index + count) else { break };
        if matches!(unit, 0 | CHAR_BACKSPACE | CHAR_LINEFEED | CHAR_CARRIAGE_RETURN | WIDE_CHAR | NARROW_CHAR) {
            break;
        }
        count += 1;
        width += 1;
        if wide {
            width += 1;
            if cursor_column + width + 1 > max_column {
                width += 1;
                break;
            }
        }
    }
    (count, width)
}

/// `extern "efiapi"` trampolines bridging EFI Simple Text Output Protocol's C ABI to
/// [`SimpleTextOutputHolder`]'s safe methods.
mod trampoline {
    use super::{Char16Str, Result, SimpleTextOutputHolder, efi, simple_text_output};

    /// Recovers the `&SimpleTextOutputHolder` behind a raw protocol pointer.
    ///
    /// # Safety
    ///
    /// `this` must be null or have been produced by [`SimpleTextOutputHolder::new`] followed by
    /// installation as the protocol interface (so `this` points at that struct's first field).
    unsafe fn holder<'a>(this: *mut simple_text_output::Protocol) -> Option<&'a SimpleTextOutputHolder> {
        // SAFETY: forwarded from this function's contract.
        unsafe { (this as *const SimpleTextOutputHolder).as_ref() }
    }

    /// Reads a NUL-terminated `CHAR16` string from a raw pointer as a safe [`Char16Str`].
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null and point to a NUL-terminated sequence of `u16` values valid for
    /// reads, per `OutputString()`/`TestString()`'s `WString` contract.
    unsafe fn read_string<'a>(ptr: *mut efi::Char16) -> core::result::Result<&'a Char16Str, efi::Status> {
        if ptr.is_null() {
            return Err(efi::Status::INVALID_PARAMETER);
        }
        // SAFETY: forwarded from this function's contract.
        unsafe { Char16Str::from_ptr(ptr) }.map_err(|_| efi::Status::INVALID_PARAMETER)
    }

    fn status_of(result: Result<()>) -> efi::Status {
        match result {
            Ok(()) => efi::Status::SUCCESS,
            Err(e) => e.into(),
        }
    }

    pub(super) extern "efiapi" fn reset(
        this: *mut simple_text_output::Protocol,
        _extended_verification: efi::Boolean,
    ) -> efi::Status {
        // SAFETY: `this` is produced by `install_protocol` per this module's contract.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.reset())
    }

    pub(super) extern "efiapi" fn output_string(
        this: *mut simple_text_output::Protocol,
        w_string: *mut efi::Char16,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        // SAFETY: forwarded from `OutputString()`'s contract on `w_string`.
        let text = match unsafe { read_string(w_string) } {
            Ok(text) => text,
            Err(status) => return status,
        };
        match holder.output_string(text) {
            Ok(true) => efi::Status::SUCCESS,
            Ok(false) => efi::Status::WARN_UNKNOWN_GLYPH,
            Err(e) => e.into(),
        }
    }

    pub(super) extern "efiapi" fn test_string(
        this: *mut simple_text_output::Protocol,
        w_string: *mut efi::Char16,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        // SAFETY: as in `output_string`.
        let text = match unsafe { read_string(w_string) } {
            Ok(text) => text,
            Err(status) => return status,
        };
        if holder.test_string(text) { efi::Status::SUCCESS } else { efi::Status::UNSUPPORTED }
    }

    pub(super) extern "efiapi" fn query_mode(
        this: *mut simple_text_output::Protocol,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        if columns.is_null() || rows.is_null() {
            return efi::Status::INVALID_PARAMETER;
        }
        match holder.query_mode(mode_number) {
            Ok((c, r)) => {
                // SAFETY: `columns`/`rows` are non-null, checked above. Validity of the pointee
                // for the write is guaranteed by `QueryMode()`'s contract on its out-params.
                unsafe {
                    columns.write_unaligned(c);
                    rows.write_unaligned(r);
                }
                efi::Status::SUCCESS
            }
            Err(e) => e.into(),
        }
    }

    pub(super) extern "efiapi" fn set_mode(this: *mut simple_text_output::Protocol, mode_number: usize) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.set_mode(mode_number))
    }

    pub(super) extern "efiapi" fn set_attribute(
        this: *mut simple_text_output::Protocol,
        attribute: usize,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.set_attribute(attribute))
    }

    pub(super) extern "efiapi" fn clear_screen(this: *mut simple_text_output::Protocol) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.clear_screen())
    }

    pub(super) extern "efiapi" fn set_cursor_position(
        this: *mut simple_text_output::Protocol,
        column: usize,
        row: usize,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.set_cursor_position(column, row))
    }

    pub(super) extern "efiapi" fn enable_cursor(
        this: *mut simple_text_output::Protocol,
        visible: efi::Boolean,
    ) -> efi::Status {
        // SAFETY: as in `reset`.
        let Some(holder) = (unsafe { holder(this) }) else { return efi::Status::INVALID_PARAMETER };
        status_of(holder.enable_cursor(visible.into()))
    }
}
