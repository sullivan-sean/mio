use super::path_offset;
use crate::net::AddressKind;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

pub(crate) struct SocketAddr {
    sockaddr: libc::sockaddr_un,
    socklen: libc::socklen_t,
}

impl SocketAddr {
    pub(crate) fn address(&self) -> AddressKind<'_> {
        let offset = path_offset(&self.sockaddr);
        // Don't underflow in `len` below.
        if (self.socklen as usize) < offset {
            return AddressKind::Unnamed;
        }
        let len = self.socklen as usize - offset;
        let path = unsafe { &*(&self.sockaddr.sun_path as *const [libc::c_char] as *const [u8]) };

        // macOS seems to return a len of 16 and a zeroed sun_path for unnamed addresses
        if len == 0
            || (cfg!(not(any(target_os = "linux", target_os = "android")))
                && self.sockaddr.sun_path[0] == 0)
        {
            AddressKind::Unnamed
        } else if self.sockaddr.sun_path[0] == 0 {
            AddressKind::Abstract(&path[1..len])
        } else {
            AddressKind::Pathname(OsStr::from_bytes(&path[..len - 1]).as_ref())
        }
    }
}

cfg_os_poll! {
    use std::{io, mem};

    impl SocketAddr {
        pub(crate) fn new<F>(f: F) -> io::Result<SocketAddr>
        where
            F: FnOnce(*mut libc::sockaddr, &mut libc::socklen_t) -> io::Result<libc::c_int>,
        {
            let mut sockaddr = {
                let sockaddr = mem::MaybeUninit::<libc::sockaddr_un>::zeroed();
                unsafe { sockaddr.assume_init() }
            };

            let raw_sockaddr = &mut sockaddr as *mut libc::sockaddr_un as *mut libc::sockaddr;
            let mut socklen = mem::size_of_val(&sockaddr) as libc::socklen_t;

            f(raw_sockaddr, &mut socklen)?;
            Ok(SocketAddr::from_parts(sockaddr, socklen))
        }

        pub(crate) fn from_parts(sockaddr: libc::sockaddr_un, socklen: libc::socklen_t) -> SocketAddr {
            SocketAddr { sockaddr, socklen }
        }
    }
}
