use std::fs::File;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileStamp {
    identity: (u64, u64),
    pub(crate) size: u64,
    modified: SystemTime,
    changed: (i64, i64),
}

impl FileStamp {
    pub(crate) fn read(file: &File) -> Option<Self> {
        let metadata = file.metadata().ok()?;
        #[cfg(unix)]
        let (identity, changed) = {
            use std::os::unix::fs::MetadataExt;
            (
                (metadata.dev(), metadata.ino()),
                (metadata.ctime(), metadata.ctime_nsec()),
            )
        };
        #[cfg(windows)]
        let (identity, changed) = {
            use std::os::windows::io::AsRawHandle;
            use windows::Win32::Foundation::HANDLE;
            use windows::Win32::Storage::FileSystem::{
                BY_HANDLE_FILE_INFORMATION, FILE_BASIC_INFO, FileBasicInfo,
                GetFileInformationByHandle, GetFileInformationByHandleEx,
            };
            let handle = HANDLE(file.as_raw_handle());
            let mut information = BY_HANDLE_FILE_INFORMATION::default();
            let mut basic = FILE_BASIC_INFO::default();
            unsafe {
                GetFileInformationByHandle(handle, &raw mut information)
                    .ok()?;
                GetFileInformationByHandleEx(
                    handle,
                    FileBasicInfo,
                    (&raw mut basic).cast(),
                    size_of::<FILE_BASIC_INFO>() as u32,
                )
                .ok()?;
            }
            if basic.ChangeTime == 0 {
                return None;
            }
            (
                (
                    u64::from(information.dwVolumeSerialNumber),
                    (u64::from(information.nFileIndexHigh) << 32)
                        | u64::from(information.nFileIndexLow),
                ),
                (basic.ChangeTime, 0),
            )
        };
        Some(Self {
            identity,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
            changed,
        })
    }
}
