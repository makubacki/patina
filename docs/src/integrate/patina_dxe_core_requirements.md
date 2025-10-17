# Patina Requirements

The Patina DXE Core has several functional and implementation differences from the
[Platform Initialization (PI) Spec](https://uefi.org/specifications) and EDK II DXE Core implementation.

The Patina DXE Readiness Tool validates many of these requirements.

## Platform Requirements

Platforms should ensure the following specifications are met when transitioning over to the Patina DXE core:

### 1. Dispatcher Requirements

The following are the set of requirements the Patina DXE Core has in regard to driver dispatch.

#### 1.1 No Traditional SMM

Traditional System Management Mode (SMM) is not supported in Patina. Standalone MM is supported.

Traditional SMM is not supported to prevent coupling between the DXE and MM environments. This is error
prone, unnecessarily increases the scopes of DXE responsibilities, and can lead to security vulnerabilities.

Standalone MM should be used instead. The combined drivers have not gained traction in actual implementations due
to their lack of compatibility for most practical purposes, increased likelihood of coupling between core environments,
and user error when authoring those modules. The Patina DXE Core focuses on modern use cases and simplification of the
overall DXE environment.

This specifically means that the following SMM module types that require cooperation between the SMM and DXE
dispatchers are not supported:

- `EFI_FV_FILETYPE_SMM` (`0xA`)
- `EFI_FV_FILETYPE_SMM_CORE` (`0xD`)

Further, combined DXE modules will not be dispatched. These include:

- `EFI_FV_FILETYPE_COMBINED_PEIM_DRIVER` (`0x8`)
- `EFI_FV_FILETYPE_COMBINED_SMM_DXE` (`0xC`)

DXE drivers and Firmware volumes **will** be dispatched:

- `EFI_FV_FILETYPE_DRIVER` (`0x7`)
- `EFI_FV_FILETYPE_FIRMWARE_VOLUME_IMAGE` (`0xB`)

Because Traditional SMM is not supported, events such as the `gEfiEventDxeDispatchGuid` defined in the PI spec and used
in the EDK II DXE Core to signal the end of a DXE dispatch round so SMM drivers with DXE dependency expressions could be
reevaluated will not be signaled.

Dependency expressions such as `EFI_SECTION_SMM_DEPEX` will not be evaluated on firmware volumes.

The use of Traditional SMM and combined drivers is detected by the Patina DXE Readiness Tool, which will report
this as an issue requiring remediation before Patina can be used.

Additional resources:

- [Standalone MM Information](https://github.com/microsoft/mu_feature_mm_supv/blob/main/Docs/TraditionalAndStandaloneMm.md)
- [Traditional MM vs Standalone MM Breakdown](https://github.com/microsoft/mu_feature_mm_supv/blob/main/Docs/TraditionalAndStandaloneMm.md)
- [Porting to Standalone MM](https://github.com/microsoft/mu_feature_mm_supv/blob/main/MmSupervisorPkg/Docs/PlatformIntegration/PlatformIntegrationSteps.md#standalone-mm-changes)

> **Guidance:**
> Platforms must transition to Standalone MM (or not use MM at all, as applicable) using the provided guidance. All
> combined modules must be dropped in favor of single phase modules.

#### 1.2 A Priori Driver Dispatch Is Not Allowed

The Patina DXE Core does not support A Priori driver dispatch as described in the PI spec and supported in EDK II. See
the [Dispatcher Documentation](../dxe_core/dispatcher.md) for details and justification. Patina will dispatch drivers
in FFS listed order.

> **Guidance:**
> A Priori sections must be removed and proper driver dispatch must be ensured using depex statements. Drivers may
> produce empty protocols solely to ensure that other drivers can use that protocol as a depex statement, if required.
> Platforms may also list drivers in FFSes in the order they should be dispatched, though it is recommended to rely on
> depex statements.

#### 1.3 Driver Section Alignment Must Be a Multiple of 4 KB

Patina relies on using a 4 KB page size and as a result requires that the C based drivers it dispatches have a
multiple of 4KB as a page size in order to apply image memory protections. The EDK II DXE Core cannot apply image
memory protections on images without this section alignment requirement, but it will dispatch them, depending on
configuration.

ARM64 DXE_RUNTIME_DRIVERs must have a multiple of 64 KB image section alignment per UEFI spec requirements.
This is required to boot operating systems with 16 KB or 64 KB page sizes.

Patina components will have 4 KB section alignment by nature of being compiled into Patina.

The DXE Readiness Tool validates all drivers have a multiple of 4 KB section alignment and reports an error if
not. It will also validate that ARM64 DXE_RUNTIME_DRIVERs have a multiple of 64KB section alignment.

> **Guidance:**
> All C based drivers must be compiled with a linker flag that enforces a multiple of 4 KB section alignment.
> For MSVC, this linker flag is `/ALIGN:0x1000` for GCC/CLANG, the flag is `-z common-page-size=0x1000`. This section
> allows for multiples of 4 KB or 64 KB, depending on driver, but unless a specific use case dictates greater section
> alignment, then it is recommended to use 4 KB for everything except for ARM64 DXE_RUNTIME_DRIVERs, which should use
> 64 KB, e.g. `/ALIGN:0x10000` for MSVC and `-z common-page-size=0x10000` for GCC/CLANG.

#### 1.4 CpuDxe Is No Longer Used

EDK II supplies a driver named `CpuDxe` that provides CPU related functionality to a platform. In Patina DXE Core, this
is part of the core, not offloaded to a driver. As a result, the CPU Arch and memory attributes protocols are owned by
the Patina DXE Core. MultiProcessor (MP) Services are not part of the core. ARM64 already does not have MP Services
owned by `CpuDxe`, `ArmPsciMpServicesDxe` owns them. There is an x64 implementation of
[`MpDxe`](https://github.com/microsoft/mu_basecore/blob/HEAD/UefiCpuPkg/MpDxe/MpDxe.inf) that platforms should add to
their flash file for use with the Patina DXE Core if MP services are desired.

On ARM64 systems, when Patina assumes ownership of `CpuDxe` it also encompasses the functionality provided by
`ArmGicDxe`. This is because the GIC is considered a prerequisite for `CpuDxe` to handle interrupts correctly. As such,
ARM64 platforms also should not include `ArmGicDxe`.

> **Guidance:**
> Platforms must not include `CpuDxe` in their platforms and instead use CPU services from Patina DXE Core and MP Services
> from a separate C based driver. ARM64 platforms should not use `ArmGicDxe`.

### 2. Hand Off Block (HOB) Requirements

The following are the Patina DXE Core HOB requirements.

#### 2.1 Resource Descriptor HOB v2

Patina uses the
[Resource Descriptor HOB v2](https://github.com/OpenDevicePartnership/patina/tree/main/sdk/patina/src/pi/hob.rs),
which is in process of being added to the PI spec, instead of the
[EFI_HOB_RESOURCE_DESCRIPTOR](https://uefi.org/specs/PI/1.9/V3_HOB_Code_Definitions.html#resource-descriptor-hob).

Platforms need to exclusively use the Resource Descriptor HOB v2 and not EFI_HOB_RESOURCE_DESCRIPTOR. Functionally,
this just requires adding an additional field to the v1 structure that describes the cacheability attributes to set on
this region.

Patina requires cacheability attribute information for memory ranges because it implements full control of memory
management and cache hierarchies in order to provide a cohesive and secure implementation of memory protection. This
means that pre-DXE paging/caching setups will be superseded by Patina and Patina will rely on the Resource Descriptor
HOB v2 structures as the canonical description of memory rather than attempting to infer it from page table/cache
control state.

Patina will ignore any EFI_HOB_RESOURCE_DESCRIPTORs. The Patina DXE Readiness Tool verifies that all
EFI_HOB_RESOURCE_DESCRIPTORs produced have a v2 HOB covering that region of memory and that all of the
EFI_HOB_RESOURCE_DESCRIPTOR fields match the corresponding v2 HOB fields for that region.

The DXE Readiness Tool also verifies that a single valid cacheability attribute is set in every Resource Descriptor HOB
v2. The accepted attributes are EFI_MEMORY_UC, EFI_MEMORY_WC, EFI_MEMORY_WT, EFI_MEMORY_WB, and EFI_MEMORY_WP.
EFI_MEMORY_UCE, while defined as a cacheability attribute in the UEFI spec, is not implemented by modern architectures
and so is prohibited. The DXE Readiness Tool will fail if EFI_MEMORY_UCE is present in a v2 HOB.

> **Guidance:**
> Platforms must produce Resource Descriptor HOB v2s with a single valid cacheability attribute set. These can be the
> existing Resource Descriptor HOB fields with the cacheability attribute set as the only additional field in the v2
> HOB.

#### 2.2 MMIO and Reserved Regions Require Resource Descriptor HOB v2s

All memory resources used by the system require Resource Descriptor HOB v2s. Patina needs this information to map MMIO
and reserved regions as existing EDK II based drivers expect to be able to touch these memory types without allocating
it first; EDK II does not require Resource Descriptor HOBs for these regions.

This cannot be tested in the DXE Readiness Tool because the tool does not know what regions may be reserved or MMIO
without the platform telling it and the only mechanism for a platform to do that is through a Resource Descriptor HOB
v2. Platforms will see page faults if a driver attempts to access an MMIO or reserved region that does not have a
Resource Descriptor HOB v2 describing it.

> **Guidance:**
> Platforms must create Resource Descriptor HOB v2s for all memory resources including MMIO and reserved memory with
> a valid cacheability attribute set.

#### 2.3 Overlapping HOBs Prohibited

Patina does not allow there to be overlapping Resource Descriptor HOB v2s in the system and the DXE Readiness Tool will
fail if that is the case. Patina cannot choose which HOB should be valid for the overlapping region; the platform must
decide this and correctly build its resource descriptor HOBs to describe system resources.

The EDK II DXE CORE silently ignores overlapping HOBs, which leads to unexpected behavior when a platform believes both
HOBs or part of both HOBs, is being taken into account.

> **Guidance:**
> Platforms must produce non-overlapping HOBs by splitting up overlapping HOBs into multiple HOBs and eliminating
> duplicates.

#### 2.4 No Memory Allocation HOB for Page 0

Patina does not allow there to be a memory allocation HOB for page 0. The EDK II DXE Core allows allocations within page
0. Page 0 must be unmapped in the page table to catch null pointer dereferences and this cannot be safely done if a
driver has allocated this page.

The DXE Readiness Tool will fail if a Memory Allocation HOB is discovered that covers page 0.

> **Guidance:**
> Platforms must not allocate page 0.

### 3. Miscellaneous Requirements

This section details requirements that do not fit under another category.

#### 3.1 Exit Boot Services Memory Allocations Are Not Allowed

When `EXIT_BOOT_SERVICES` is signaled, the memory map is not allowed to change. See
[Exit Boot Services Handlers](../dxe_core/memory_management.md#exit-boot-services-handlers). The EDK II DXE Core does
not prevent memory allocations at this point, which causes hibernate resume failures, among other bugs.

The DXE Readiness Tool is not able to detect this anti-pattern because it requires driver dispatching and specific target
configurations to trigger the memory allocation/free.

> **Guidance:**
> Platforms must ensure all memory allocations/frees take place before exit boot services callbacks.

#### 3.2 All Code Must Support Native Address Width

By default, the Patina DXE Core allocates memory top-down in the available memory space. Other DXE implementations such
as EDK II allocate memory bottom-up. This means that all code storing memory addresses (including C and Rust code) must
support storing that address in a variable as large as the native address width.

For example, in a platform with free system memory > 4GB, EDK II may have never returned a buffer address greater than
4GB so `UINT32` variables were sufficient (though bad practice) for storing the address. Since Patina allocates memory
top-down, addresses greater than 4GB will be returned if suitable memory is available in that range and code must
be able to accommodate that.

> **Guidance:**
> All DXE code (including all C modules) must support storing native address width memory addresses.

Note: The Patina DXE Readiness Tool does not perform this check.

#### 3.3 ConnectController() Must Explicitly Be Called For Handles Created/Modified During Image Start

To maintain compatibility with UEFI drivers that are written to the EFI 1.02 Specification, the EDK II `StartImage()`
implementation is extended to monitor the handle database before and after each image is started. If any handles are
created or modified when an image is started, then `EFI_BOOT_SERVICES.ConnectController()` is called with the
`Recursive` parameter set to `TRUE` for each of the newly created or modified handles before `StartImage()` returns.

Patina does not implement this behavior. Images and platforms dependent on this behavior will need to be modified to
explicitly call `ConnectController()` on any handles that they create or modify.

### 4. Known Limitations

This section details requirements Patina currently has due to limitations in implementation, but that support will be
added for in the future. Currently, this section is empty.
