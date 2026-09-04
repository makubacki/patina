//! The graphics console component.
//!
//! [`GraphicsConsoleProvider`] installs an EFI Driver Binding Protocol (`GraphicsConsoleDriverBinding`)
//! that binds a `Simple Text Output` console to every Graphics Output Protocol (GOP) controller in the
//! system. Each controller it starts gets its own `SimpleTextOutputHolder`, rendering text with the
//! EFI HII Font Protocol over the GOP.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use alloc::boxed::Box;
use core::ptr::NonNull;

use patina::{
    char16,
    component::{
        component,
        service::{
            Service,
            pcd::PcdServices,
            uefi_services::{
                driver_model::{
                    component_name::UefiDriverModelComponentName,
                    driver_binding::DriverBinding,
                    language::{LanguageEntry, LanguageTable},
                },
                handle::Handle,
                protocol::{OpenAttributes, ProtocolError, ProtocolServices, ProtocolServicesExt},
                tpl::TplServices,
            },
        },
    },
    error::Result,
    protocol::ProtocolInterface,
    standard::efi::protocols::{device_path, graphics_output, hii_database, hii_font},
    uefi::device_path::walker::DevicePathWalker,
};

use crate::console::{
    font::HiiFontHandle, font_package, gop::GopHandle, output::SimpleTextOutputHolder, pcd::ConsolePreferences,
};

/// The name this driver publishes through the EFI Component Name protocols.
static DRIVER_NAME: LanguageTable =
    LanguageTable(&[LanguageEntry { iso639: b"eng", rfc4646: "en", name: char16!("Graphics Console") }]);

/// Installs the graphics console's driver binding.
///
/// `Service<dyn PcdServices>` is an optional dependency. A platform with no PCD driver gets a console
/// that defaults to the display's highest resolution and largest text mode.
#[derive(Default)]
pub struct GraphicsConsoleProvider;

#[component]
impl GraphicsConsoleProvider {
    /// Creates a new instance of the component.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(
        self,
        protocols: Service<dyn ProtocolServices>,
        tpl: Service<dyn TplServices>,
        pcd: Option<Service<dyn PcdServices>>,
    ) -> Result<()> {
        let binding = GraphicsConsoleDriverBinding { protocols, tpl, pcd };
        protocols.install_driver_binding(binding)?;
        Ok(())
    }
}

/// The EFI Driver Binding implementation. Binds to every Graphics Output Protocol
/// controller and installs a [`SimpleTextOutputHolder`] over each one it starts.
struct GraphicsConsoleDriverBinding {
    protocols: Service<dyn ProtocolServices>,
    tpl: Service<dyn TplServices>,
    pcd: Option<Service<dyn PcdServices>>,
}

impl UefiDriverModelComponentName for GraphicsConsoleDriverBinding {
    fn driver_name(&self) -> &'static LanguageTable {
        &DRIVER_NAME
    }
}

impl DriverBinding for GraphicsConsoleDriverBinding {
    fn supported(
        &self,
        agent: Handle,
        controller: Handle,
        _remaining_device_path: Option<DevicePathWalker>,
    ) -> core::result::Result<(), ProtocolError> {
        // Requiring a real device path keeps this driver from binding on top of a virtual,
        // aggregate GOP handle (for example one con splitter produces).  Both opens are
        // dropped (closed) at the end of this function. `Start()` re-opens whatever it actually needs.
        let _device_path = self.protocols.open_protocol::<device_path::Protocol>(
            controller,
            agent,
            OpenAttributes::ByDriver { controller },
        )?;
        let _gop = self.protocols.open_protocol::<graphics_output::Protocol>(
            controller,
            agent,
            OpenAttributes::ByDriver { controller },
        )?;

        // A console with no font source to rasterize with is not useful, so require one to be
        // present before binding.
        self.protocols.locate_protocol::<hii_font::Protocol>().map_err(|_| ProtocolError::NotFound)?;
        Ok(())
    }

    fn start(
        &self,
        agent: Handle,
        controller: Handle,
        _remaining_device_path: Option<DevicePathWalker>,
    ) -> core::result::Result<(), ProtocolError> {
        let gop_ptr = self.protocols.open_interface(
            controller,
            <graphics_output::Protocol as ProtocolInterface>::PROTOCOL_GUID,
            agent,
            OpenAttributes::ByDriver { controller },
        )?;
        // Note: This is a closure so every early-return path below still releases the GOP usage on failure.
        let result = (|| {
            let gop = NonNull::new(gop_ptr.as_raw().cast::<graphics_output::Protocol>())
                .ok_or(ProtocolError::InvalidParameter)?;
            // SAFETY: `gop_ptr` was just returned by `open_interface` for `graphics_output::Protocol`,
            // whose `ProtocolInterface` impl guarantees the interface has that layout. It stays
            // open (and this pointer valid) until `stop()` closes it.
            let gop = unsafe { GopHandle::new(gop) };

            let hii_font_ptr =
                self.protocols.locate_interface(<hii_font::Protocol as ProtocolInterface>::PROTOCOL_GUID)?;
            let hii_font =
                NonNull::new(hii_font_ptr.as_raw().cast::<hii_font::Protocol>()).ok_or(ProtocolError::NotFound)?;
            // SAFETY: as above, for `hii_font::Protocol`. HII Font is a system-wide service (not
            // opened against a controller), so it is only located, never closed in `stop()`.
            let hii_font = unsafe { HiiFontHandle::new(hii_font) };

            // Note: The console still works without a font package, just without any glyphs to render until
            // something else supplies a font package.
            if let Ok(hii_database) = self.protocols.locate_protocol::<hii_database::Protocol>()
                && let Err(err) = font_package::register(hii_database)
            {
                log::warn!("Failed to register default HII font package: {err:?}");
            }

            let preferences = ConsolePreferences::read(self.pcd);
            let holder =
                SimpleTextOutputHolder::new(gop, hii_font, self.tpl, preferences.resolution, preferences.text_mode)
                    .map_err(|_| ProtocolError::InvalidParameter)?;

            self.protocols.install_protocol::<SimpleTextOutputHolder>(Some(controller), holder)?;
            Ok(())
        })();

        if result.is_err() {
            let _ = self.protocols.close_interface(
                controller,
                <graphics_output::Protocol as ProtocolInterface>::PROTOCOL_GUID,
                agent,
                Some(controller),
            );
        }
        result
    }

    fn stop(&self, agent: Handle, controller: Handle, _children: &[Handle]) -> core::result::Result<(), ProtocolError> {
        let interface = self
            .protocols
            .interface_on_handle(controller, <SimpleTextOutputHolder as ProtocolInterface>::PROTOCOL_GUID)?;
        self.protocols.uninstall_interface(
            controller,
            <SimpleTextOutputHolder as ProtocolInterface>::PROTOCOL_GUID,
            interface,
        )?;

        // SAFETY: `interface` was produced by `install_protocol::<SimpleTextOutputHolder>` in
        // `start()`, which leaked a `Box<SimpleTextOutputHolder>` at this address. The uninstall
        // above just succeeded, so no other code can reach this pointer through the protocol
        // database anymore, making it safe to reclaim and drop.
        drop(unsafe { Box::from_raw(interface.as_raw().cast::<SimpleTextOutputHolder>()) });

        self.protocols.close_interface(
            controller,
            <graphics_output::Protocol as ProtocolInterface>::PROTOCOL_GUID,
            agent,
            Some(controller),
        )
    }
}
