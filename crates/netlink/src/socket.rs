// https://tools.ietf.org/html/rfc3549
// /usr/include/linux/netlink.h

use libc;

use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};


pub const AF_NETLINK: u8   = 16;
pub const SOL_NETLINK: i32 = 270;
pub const NETLINK_ADD_MEMBERSHIP: libc::c_int   = 1;
pub const NETLINK_DROP_MEMBERSHIP: libc::c_int  = 2;
pub const NETLINK_PKTINFO: libc::c_int          = 3;
pub const NETLINK_BROADCAST_ERROR: libc::c_int  = 4;
pub const NETLINK_NO_ENOBUFS: libc::c_int       = 5;
pub const NETLINK_RX_RING: libc::c_int          = 6;
pub const NETLINK_TX_RING: libc::c_int          = 7;
pub const NETLINK_LISTEN_ALL_NSID: libc::c_int  = 8;
pub const NETLINK_LIST_MEMBERSHIPS: libc::c_int = 9;
pub const NETLINK_CAP_ACK: libc::c_int          = 10;


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sockaddr_nl {
    pub nl_family: u16,     // AF_NETLINK
    pub nl_pad: u16,        // zero
    pub nl_pid: u32,        // port ID
    pub nl_groups: u32,     // multicast groups mask
}


#[derive(Debug)]
pub struct NetlinkSocket {
    fd: libc::c_int,
}

impl NetlinkSocket {
    pub fn new(proto: i32) -> Result<Self, io::Error> {
        // http://man7.org/linux/man-pages/man7/netlink.7.html
        // 
        // Netlink is a datagram-oriented service.  Both SOCK_RAW and SOCK_DGRAM
        // are valid values for socket_type.  However, the netlink protocol does
        // not distinguish between datagram and raw sockets.
        let fd = unsafe { libc::socket(AF_NETLINK as i32, libc::SOCK_RAW, proto) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { fd })
    }

    pub fn bind(&mut self, pid: u32, groups: u32) -> Result<(), io::Error> {
        let nladdr = sockaddr_nl {
            nl_family: AF_NETLINK as u16,
            nl_pad   : 0,
            nl_pid   : pid,
            nl_groups: groups,
        };

        let nladdr_ptr = &nladdr as *const sockaddr_nl as  *const libc::sockaddr;
        let sa_len = std::mem::size_of::<sockaddr_nl>() as u32;

        if unsafe { libc::bind(self.fd, nladdr_ptr, sa_len) } < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    #[inline]
    pub fn flags(&self) -> Result<i32, io::Error> {
        let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL, 0) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        
        Ok(flags)
    }
    
    #[inline]
    pub fn set_flags(&mut self, flags: i32) -> Result<(), io::Error> {
        if unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Set underlying socket file descriptor to be blocking
    #[inline]
    pub fn set_block(&mut self) -> Result<(), io::Error> {
        self.set_flags(self.flags()? & !libc::O_NONBLOCK)
    }

    /// Set underlying socket file descriptor to be non blocking
    #[inline]
    pub fn set_nonblock(&mut self) -> Result<(), io::Error> {
        self.set_flags(self.flags()? | libc::O_NONBLOCK)
    }

    /// Determines if underlying file descriptor is blocking - `Stream` feature will throw an
    /// error if this function returns false
    #[inline]
    pub fn is_blocking(&self) -> Result<bool, io::Error> {
        Ok((self.flags()? & libc::O_NONBLOCK) == 0)
    }

    /// Set multicast groups for socket
    pub fn set_mcast_groups(&mut self, groups: u32) -> Result<(), io::Error> {
        // nl_pktinfo
        let groups_ptr = &groups as *const u32 as *const libc::c_void;
        let groups_len = std::mem::size_of::<u32>() as libc::socklen_t;
        let ret = unsafe {
            libc::setsockopt(self.fd, SOL_NETLINK, NETLINK_ADD_MEMBERSHIP, groups_ptr, groups_len)
        };

        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub fn send(&mut self, buf: &[u8], flags: i32) -> Result<usize, io::Error> {
        let buf_ptr = buf.as_ptr() as *const libc::c_void;
        let buf_len = buf.len();

        let amt = unsafe { libc::send(self.fd, buf_ptr, buf_len, flags) };
        if amt < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(amt as usize)
    }

    pub fn recv(&mut self, buf: &mut [u8], flags: i32) -> Result<usize, io::Error> {
        let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;
        let buf_len = buf.len();

        let amt = unsafe { libc::recv(self.fd, buf_ptr, buf_len, flags) };
        if amt < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(amt as usize)
    }

    pub fn send2<T: Sized>(&mut self, buf: &T) -> Result<usize, io::Error> {
        let buf_len = std::mem::size_of::<T>();
        let buf_ptr = buf as *const T as *const u8;

        let buf = unsafe { std::slice::from_raw_parts(buf_ptr, buf_len) };

        let amt = self.send(buf, 0)?;
        assert_eq!(buf.len(), amt);

        Ok(amt)
    }

    pub fn recv2<T: Sized>(&mut self, buf: &mut T) -> Result<usize, io::Error> {
        let buf_len = std::mem::size_of::<T>();
        let buf_ptr = buf as *mut T as *mut u8;

        let buf = unsafe { std::slice::from_raw_parts_mut(buf_ptr, buf_len) };
        let amt = self.recv(buf, 0)?;
        assert_eq!(buf_len, amt);
        
        Ok(amt)
    }
}


impl AsRawFd for NetlinkSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for NetlinkSocket {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

impl Read for NetlinkSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv(buf, 0)
    }
}

impl Write for NetlinkSocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.send(buf, 0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for NetlinkSocket {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
        trace!("close({})", self.fd);
    }
}
