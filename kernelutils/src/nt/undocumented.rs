use core::ffi::c_void;
use core::ptr::null_mut;
use wdk_sys::{OBJECT_ATTRIBUTES, UNICODE_STRING};

#[allow(non_snake_case)]
pub fn InitializeObjectAttributes(
    object_name: Option<*mut UNICODE_STRING>,
    attributes: u32,
    root_directory: Option<*mut c_void>,
    security_descriptor: Option<*mut c_void>,
    security_quality_of_service: Option<*mut c_void>
) -> OBJECT_ATTRIBUTES {
    OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: root_directory.unwrap_or(null_mut()),
        ObjectName: object_name.unwrap_or(null_mut()),
        Attributes: attributes,
        SecurityDescriptor: security_descriptor.unwrap_or(null_mut()),
        SecurityQualityOfService: security_quality_of_service.unwrap_or(null_mut())
    }
}