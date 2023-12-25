
use bit_field::BitField;
use raw_cpuid::CpuId;
use x86::bits64::vmx;
use x86_64::registers::control::{Cr0, Cr4, Cr4Flags};
mod msr;
use msr::{FeatureControl, FeatureControlFlags, Msr, VmxBasic};
use crate::mm::{frame::alloc_page, PAGE_SIZE, address::phys_to_virt};

// 输出本机主处理器的基本参数
pub fn print_cpuid() {
    let cpuid = CpuId::new();

    if let Some(info) = cpuid.into() {
        println!("{:#?}", info);
    }

}

/// Checks if VT-x (vmx) is supported by our hardware.
pub fn has_hardware_support() -> bool {
    if let Some(feature) = CpuId::new().get_feature_info() {
        feature.has_vmx()
    } else {
        false
    }
}

pub fn hardware_enable() {
    if !has_hardware_support() {
        println!("CPU does not support feature VMX");
        return;
    } else {
        println!("CPU supports feature VMX");
    }
    // 检查该处理器的 VMX 功能是否已经打开
    if Cr4::read().contains(Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS) {
        println!("VMX is already turned on");
        return;
    } else {
        println!("VMX is turned off");
    }

    // Enable VMXON, if required.
    let ctrl = FeatureControl::read();
    let locked = ctrl.contains(FeatureControlFlags::LOCKED);
    let vmxon_outside = ctrl.contains(FeatureControlFlags::VMXON_ENABLED_OUTSIDE_SMX);
    if !locked {
        FeatureControl::write(
            ctrl | FeatureControlFlags::LOCKED | FeatureControlFlags::VMXON_ENABLED_OUTSIDE_SMX,
        )
    } else if !vmxon_outside {
        println!("VMX disabled by BIOS");
        return;
    }

    // Check control registers are in a VMX-friendly state. (SDM Vol. 3C, Appendix A.7, A.8)
    macro_rules! cr_is_valid {
        ($value: expr, $crx: ident) => {{
            use Msr::*;
            let value = $value;
            let fixed0 = concat_idents!(IA32_VMX_, $crx, _FIXED0).read();
            let fixed1 = concat_idents!(IA32_VMX_, $crx, _FIXED1).read();
            (!fixed0 | value != 0) && (fixed1 | !value != 0)
        }};
    }
    if !cr_is_valid!(Cr0::read().bits(), CR0) {
        println!("host CR0 is not valid in VMX operation");
        return;
    }
    if !cr_is_valid!(Cr4::read().bits(), CR4) {
        println!("host CR4 is not valid in VMX operation");
        return;
    }

    // Get VMCS revision identifier in IA32_VMX_BASIC MSR.
    let vmx_basic = VmxBasic::read();
    
    let vmcs_revision_id = vmx_basic.revision_id;
    let start_paddr = unsafe { alloc_page().unwrap() };
    unsafe { core::ptr::write_bytes(phys_to_virt(start_paddr) as *mut u8, 0, PAGE_SIZE) }
    unsafe {
        (*(phys_to_virt(start_paddr) as *mut u32))
            .set_bits(0..=30, vmcs_revision_id)
            .set_bit(31, false);
    }

    unsafe {
        // Enable VMX using the VMXE bit.
        Cr4::write(Cr4::read() | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS);
        // Execute VMXON.
        vmx::vmxon(start_paddr as _).unwrap();
    }
    println!("Successed to turn on VMX.");

}

pub fn hardware_disable() {
    if !Cr4::read().contains(Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS) {
        println!("VMX is turned off");
        return;
    }

    unsafe {
        // Execute VMXOFF.
        vmx::vmxoff().unwrap();
        // Remove VMXE bit in CR4.
        Cr4::update(|cr4| cr4.remove(Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS));
    };
    println!("Successed to turn off VMX.");
}

pub fn run() -> (){
    // 输出本机主处理器的基本参数
    print_cpuid();
    hardware_enable();
    // 在 VMX Root 模式下输出 Hello,world
    println!("Hello World!");
    hardware_disable();
}
