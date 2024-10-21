use alloc::format;
use alloc::vec::Vec;
use core::arch::asm;
use core::mem::zeroed;
use core::{ptr, slice};
use core::ptr::{null_mut, slice_from_raw_parts};
use wdk_sys::ntddk::{MmMapIoSpace, ZwClose, ZwMapViewOfSection, ZwOpenSection, ZwUnmapViewOfSection};
use wdk_sys::{GUID, LARGE_INTEGER, NT_SUCCESS, OBJ_CASE_INSENSITIVE, PAGE_NOCACHE, PAGE_READWRITE, PVOID, SECTION_MAP_READ};
use wdk_sys::_SECTION_INHERIT::ViewShare;
use crate::nt::undocumented::InitializeObjectAttributes;
use crate::str_to_unicode;


#[repr(C)]
#[repr(align(1))]
#[allow(non_camel_case_types)]
struct smbios_entry_point_struct {
    anchor_string: u32,
    entry_point_checksum: u8,
    entry_point_length: u8,
    major_version: u8,
    minor_version: u8,
    max_struct_size: u16,
    revision: u8,
    formated_area: [u8; 5],
    intermediate_string: [u8; 5],
    intermediate_checksum: u8,
    struct_table_length: u16,
    struct_table_address: u32,
    no_of_structures: u16,
    bcd_revision: u8,

}


#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct smbios_struct {
    r#type: u8,
    length: u8,
    handle: u16,
    subtype: u8,
}
#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct dmibios_table_entry_struct {
    size: u16,
    handle: u16,
    procedure: u32,

}

#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct dmibios_entry_point_struct {
    signature: [u8; 10],
    revision: u8,
    entry: [dmibios_table_entry_struct; 1],
}
#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]

struct SMBIOS_HEADER {
    Type: u8,
    Length: u8,
    Handle: [u8; 2],

}
pub type PSMBIOS_HEADER = *mut SMBIOS_HEADER;
pub type SMBIOS_STRING = u8;


#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct SMBIOS_TYPE0
{
    Hdr: SMBIOS_HEADER,
    Vendor: SMBIOS_STRING,
    BiosVersion: SMBIOS_STRING,
    BiosSegment: [u8; 2],
    BiosReleaseDate: SMBIOS_STRING,
    BiosSize: u8,
    BiosCharacteristics: [u8; 8],
}
#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct SMBIOS_TYPE1 {
    Hdr: SMBIOS_HEADER,
    Manufacturer: SMBIOS_HEADER,
    ProductName: SMBIOS_HEADER,
    Version: SMBIOS_HEADER,
    SerialNumber: SMBIOS_HEADER,
    Uuid: GUID,
    WakeUpType: u8,

}
#[allow(non_camel_case_types)]
#[repr(C)]
#[repr(align(1))]
struct SMBIOS_TYPE2 {
    Hdr: SMBIOS_HEADER,
    Manufacturer: SMBIOS_HEADER,
    ProductName: SMBIOS_HEADER,
    Version: SMBIOS_HEADER,
    SerialNumber: SMBIOS_HEADER,

}
#[allow(non_camel_case_types)]
#[repr(align(1))]
#[repr(C)]
struct SMBIOS_TYPE3 {
    Hdr: SMBIOS_HEADER,
    Manufacturer: SMBIOS_HEADER,
    Type: u8,
    ProductName: SMBIOS_HEADER,
    Version: SMBIOS_HEADER,
    SerialNumber: SMBIOS_HEADER,
    BootupState: u8,
    PowerSupplyState: u8,
    ThermalState: u8,
    SecurityStatus: u8,
    OemDefined: [u8; 4],

}

// bitfield::bitfield! {
//     // #[derive(Clone, Copy)]
//     pub struct PROCESSOR_SIGNATURE(u32);
//     // impl Debug;
//     pub ProcessorSteppingId: 4;
//     pub ProcessorModel: 4;
//     pub ProcessorFamily: 4;
//     pub ProcessorType: 2;
//     pub ProcessorReserved1: 2;
//     pub ProcessorXModel: 4;
//     pub ProcessorXFamily: 8;
//     pub ProcessorReserved2: 4;
//
// }


// RandomizeSerialNumber function
// fn randomize_serial_number(str: &mut [u8]) {
//     const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
//     let charset_size = CHARSET.len() as u32;
//
//     // Seed random number generator
//     let seed = unsafe { get_system_time() } & 0xFFFFFFFF;
//     let mut rand_seed = seed as u32;
//
//     for byte in str.iter_mut().take(str.len() - 1) { // Leave space for null terminator
//         rand_seed = rtl_random_ex(rand_seed);
//         *byte = CHARSET[(rand_seed % charset_size) as usize];
//     }
//     str[str.len() - 1] = 0; // Null-terminate
// }

// Placeholder functions for system calls
unsafe fn get_system_time() -> i64 {
    // Implement time retrieval
    0
}
unsafe fn smbios_check_entry_point(addr: PVOID) -> u8 {
    let entry_point_struct = unsafe { &*(addr as *mut smbios_entry_point_struct) };
    let length = entry_point_struct.entry_point_length;
    let mut checksum = 0;

    let start = addr as *mut u8;
    for i in 0..length {
        checksum += unsafe { *start.add(i as usize) };
    }

    checksum
}
unsafe fn smbios_find_entry_point(base: PVOID, n_len: usize) -> *mut smbios_entry_point_struct {
    let mut entry_point: *mut smbios_entry_point_struct = null_mut();
    let mut temp = base as *mut u32;

    while entry_point.is_null()
        && (temp < unsafe { base.add(0x10000) as *mut u32 })
        && ((temp as *mut u8) < (base as *mut u8).add(n_len)) {
        if unsafe { *temp == 0x5F4D535F } {
            if smbios_check_entry_point(temp as _) == 0 {
                entry_point = temp as *mut smbios_entry_point_struct;
                log::warn!("SM-BIOS V{}.{} entry point found at {:?}",
                         unsafe { (*entry_point).major_version },
                         unsafe { (*entry_point).minor_version },
                         temp);
            }
        }
        temp = unsafe { temp.add(1) }; // Increment by 4 bytes
    }
    entry_point
}

// dmibios_find_entry_point function
unsafe fn dmibios_find_entry_point(base: PVOID, n_len: usize) -> *mut dmibios_entry_point_struct {
    let mut entry_point: *mut dmibios_entry_point_struct = null_mut();
    let bios_signature: [u8; 10] = [0x5f, 0x44, 0x4d, 0x49, 0x32, 0x30, 0x5f, 0x4e, 0x54, 0x5f];
    let mut temp = base as *mut u8;

    while entry_point.is_null()
        && (temp < unsafe { base.add(0x10000) as *mut u8 }.wrapping_sub(bios_signature.len() as usize + 32))
        && ((temp as *mut u8) < (base as *mut u8).add(n_len)) {
        let temp_dword = temp as *mut u32;

        if unsafe { *temp_dword == 0x494d445f } {
            entry_point = temp as *mut dmibios_entry_point_struct;

            log::warn!("DMI-BIOS revision {} entry point at {:?}",
                     unsafe { (*entry_point).revision },
                     temp);

            if unsafe { ptr::read(temp as *const [u8; 10]) } == bios_signature {
                log::warn!("DMI BIOS successfully identified");
            }
        }
        temp = unsafe { temp.add(1) };
    }
    entry_point
}


// Placeholder constants
const DMI_STRING: &[u8] = b"DMI\0"; // Adjust as necessary


pub fn map_physical_memory() -> Option<Vec<u8>> {
    let mut strPH = str_to_unicode("\\device\\physicalmemory");
    let mut so: LARGE_INTEGER = unsafe { zeroed() };
    unsafe { so.u.LowPart = 0x000f0000; }
    unsafe { so.u.HighPart = 0x00000000; }
    let mut view_size = 0xffff;

    let mut ba: PVOID = unsafe { null_mut() };
    let mut obj_attr = InitializeObjectAttributes
        (Some(&mut strPH.to_unicode()), OBJ_CASE_INSENSITIVE, None, None, None);
    let mut section_handle = null_mut();
    let mut status = unsafe {
        ZwOpenSection(&mut section_handle,
                      SECTION_MAP_READ, &mut obj_attr)
    };

    if !NT_SUCCESS(status) {
        return None;
    }
    unsafe {
        status = ZwMapViewOfSection(
            section_handle,
            0xFFFFFFFFFFFFFFFF as *mut core::ffi::c_void,
            &mut ba,
            0,
            0xFFFF,
            &mut so,
            &mut view_size,
            ViewShare,
            0,
            PAGE_NOCACHE | PAGE_READWRITE,
        );
    }

    if !NT_SUCCESS(status) {
        return None;
    }
    unsafe { asm!("int 3"); }
    // 0x0000021f860d0000
    log::warn!("src map address ==>   {:#x} ",ba as usize);

    let buffer = unsafe { slice::from_raw_parts(ba as *const u8, view_size as usize) };
    let vec = buffer.to_vec();
    let _ = unsafe { ZwUnmapViewOfSection(0xFFFFFFFFFFFFFFFF as *mut core::ffi::c_void, ba) };
    let _ = unsafe { ZwClose(section_handle) };
    Some(vec)
}


pub fn find_bios() {
    let src = map_physical_memory();
    if src.is_none() {
        return;
    }

    unsafe { asm!("int 3"); }
    let data: Vec<u8> = src.unwrap();
    let len = data.len();
    //src map address ==>   0x21f860d0000 0x0000021f860d0000
    for i in (0..len).step_by(4) {
        if i + 4 <= len {
            let chunk = &data[i..i + 4]; // 获取 4 字节切片
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]); // 转换为 u32
            if value == 0x5F4D535F {
                unsafe { asm!("int 3") };
                log::warn!("i= {}",i);
            }
        }
    }
}
// src.iter().for_each(|e| {
//     log::warn!("{:#x}",e);
// })
