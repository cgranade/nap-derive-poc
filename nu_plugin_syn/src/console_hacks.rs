use cfg_if;

use windows::Win32::Storage::FileSystem::CreateFileW;
use windows::Win32::System::Console::{SetStdHandle, STD_INPUT_HANDLE};
use windows::{w, core::PCWSTR};
use windows::Win32::{
    System::Console::{FreeConsole, AttachConsole, ATTACH_PARENT_PROCESS},
    Storage::FileSystem::{FILE_SHARE_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL},
    Foundation::{GENERIC_READ, GENERIC_WRITE}
};

pub fn reset_stdin() {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            // Once we have read the call, we want to detach the redirected stdin stream
            // and reattach to interactive input.
            // See https://stackoverflow.com/q/21779818 for why this works.
            unsafe {
                if !FreeConsole().as_bool() {
                    panic!("Failed to free console.");
                }
                if !AttachConsole(ATTACH_PARENT_PROCESS).as_bool() {
                    panic!("Failed to attach parent console.");
                }

                // Once we've reattached, we need to open that console's stdin
                // as a file, then set that as the new stdin handle for future
                // reads. See https://learn.microsoft.com/en-us/windows/console/setstdhandle#remarks
                // for details.
                let conin: PCWSTR = w!("CONIN$").into();
                let stdin_handle = CreateFileW(
                    conin,
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_READ,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None
                ).unwrap();
                SetStdHandle(STD_INPUT_HANDLE, stdin_handle);
            }
        }
    }
}
