use crate::Value;

use libc;

// use std::ptr;
use std::str::FromStr;
use std::io::{self, Read, Write};
use std::fs::{OpenOptions, ReadDir,};
use std::path::{Path, PathBuf,};


// largest number of components supported
pub const CTL_MAXNAME: usize = 10;

#[cfg(target_os = "linux")]
const PATH_PREFIX: &'static str = "/proc/sys";
#[cfg(not(target_os = "linux"))]
const PATH_PREFIX: &'static str = concat!(env!("PWD"), "/proc/sys");

#[cfg(target_os = "linux")]
const ROOT_PATH: &'static str = "/proc/sys/kernel";
#[cfg(not(target_os = "linux"))]
const ROOT_PATH: &'static str = concat!(env!("PWD"), "/proc/sys/kernel");

// Top-level names
pub const CTL_KERN: libc::c_int = 1; // General kernel info and control
pub const CTL_VM: libc::c_int = 2; // VM management
pub const CTL_NET: libc::c_int = 3; // Networking
pub const CTL_PROC: libc::c_int = 4; // removal breaks strace(1) compilation
pub const CTL_FS: libc::c_int = 5; // Filesystems
pub const CTL_DEBUG: libc::c_int = 6; // Debugging
pub const CTL_DEV: libc::c_int = 7; // Devices
pub const CTL_BUS: libc::c_int = 8; // Busses
pub const CTL_ABI: libc::c_int = 9; // Binary emulation
pub const CTL_CPU: libc::c_int = 10; // CPU stuff (speed scaling, etc)
pub const CTL_ARLAN: libc::c_int = 254; // arlan wireless driver
pub const CTL_S390DBF: libc::c_int = 5677; // s390 debug
pub const CTL_SUNRPC: libc::c_int = 7249; // sunrpc debug
pub const CTL_PM: libc::c_int = 9899; // frv power management
pub const CTL_FRV: libc::c_int = 9898; // frv specific sysctls


// TODO:
// Metadata Table
pub const TABLE: &[(&'static str, Kind)] = &[
    ("kernel", Kind::Node,),
    ("kernel.ostype", Kind::I32,),
    ("kernel.version", Kind::I32,),
    ("kernel.osrelease", Kind::String,),
];


#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Node,
    String,
    Struct,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Unknow,
}

#[derive(Debug)]
pub struct Metadata {
    kind: Kind,
    // format
    indication: &'static str,
}


#[derive(Debug, Clone)]
pub struct Mib {
    path: PathBuf,
}

impl Mib {
    #[inline]
    pub fn components(&self) -> &[libc::c_int] {
        // &self.inner[..self.len]
        unimplemented!()
    }

    pub fn name(&self) -> Result<String, io::Error> {
        let name = self.path.strip_prefix(PATH_PREFIX)
                        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("{}", e)))?
                        .to_str()
                        .ok_or(io::Error::new(io::ErrorKind::Other, "Not a utf-8 seqs"))?;
        Ok(name.replace("/", "."))
    }
    
    // Get Value by Mib
    pub fn value(&self) -> Result<Vec<u8>, io::Error> {
        if !self.path.is_file() {
            return Err(io::Error::new(io::ErrorKind::Other, "Can not get value from a Node."));
        }

        let mut file = OpenOptions::new().read(true).write(false).open(&self.path)?;
        let mut val = Vec::new();
        file.read_to_end(&mut val)?;
        Ok(val)
    }

    // Set Value By Mib
    pub fn set_value(&self, val: &[u8]) -> Result<Vec<u8>, io::Error> {
        let mut file = OpenOptions::new().read(false).write(true).open(&self.path)?;
        file.write_all(val)?;
        self.value()
    }

    // Get metadata ( ValueKind )
    pub fn metadata(&self) -> Result<Metadata, io::Error> {
        unimplemented!()
    }

    #[inline]
    pub fn description(&self) -> Result<String, std::io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "Description not available on Linux"))
    }

    pub fn iter(&self) -> Result<MibIter, io::Error> {
        MibIter::new(&self.path)
    }
}



impl FromStr for Mib {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = if s.starts_with(PATH_PREFIX) {
            if s.ends_with(PATH_PREFIX) {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            PathBuf::from(s)
        } else {
            PathBuf::from(PATH_PREFIX).join(s.replace(".", "/"))
        };
        
        // return absolute path, and ensure the path is exists.
        let path = path.canonicalize()?;

        debug_assert!(path.is_absolute());
        debug_assert!(path.exists());
        debug_assert!(path.starts_with(PATH_PREFIX));

        Ok(Self { path, })
    }
}

impl Default for Mib {
    fn default() -> Self {
        Self {
            path: PathBuf::from(ROOT_PATH)
        }
    }
}

#[derive(Debug)]
pub struct MibIter {
    dirs: Vec<ReadDir>,
}

impl MibIter {
    fn new(path: &Path) -> Result<Self, io::Error> {
        let root = Path::new(PATH_PREFIX);
        debug_assert!(root.is_dir());

        let mut dirs = Vec::new();
        dirs.push(root.read_dir()?);

        fn seek(dirs: &mut Vec<ReadDir>, stop_path: &Path) -> Result<(), io::Error> {
            if dirs.len() == 0 {
                return Ok(());
            }

            let idx = dirs.len() - 1;
            let dir = match dirs.get_mut(idx) {
                Some(dir) => dir,
                None => return Ok(()),
            };
            
            loop {
                let entry = dir.next();
                if entry.is_none() {
                    dirs.remove(idx);
                    return seek(dirs, stop_path);
                }

                let entry = entry.unwrap()?;
                let file_type = entry.file_type()?;
                let file_path = entry.path();
                
                if file_type.is_dir() {
                    dirs.push(file_path.read_dir()?);
                    if file_path == stop_path {
                        break;
                    }

                    return seek(dirs, stop_path);

                } else if file_type.is_file() {
                    // println!("Skip: {:?}", file_path);
                    if file_path == stop_path {
                        break;
                    }
                } else {
                    // TODO: symlink
                    unimplemented!()
                }
            }

            Ok(())
        }

        seek(&mut dirs, &path)?;
        
        Ok(MibIter {
            dirs: dirs,
        })
    }
}

impl Iterator for MibIter {
    type Item = Result<Mib, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.dirs.len() == 0 {
            return None;
        }

        let idx = self.dirs.len() - 1;
        let dir = self.dirs.get_mut(idx).unwrap();

        match dir.next() {
            Some(Ok(entry)) => {
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(e) => return Some(Err(e)),
                };
                let file_path = entry.path();
                
                if file_type.is_dir() {
                    match file_path.read_dir() {
                        Ok(sub_dir) => self.dirs.push(sub_dir),
                        Err(e) => return Some(Err(e)),
                    }
                    self.next()
                } else if file_type.is_file() {
                    let s = file_path.to_string_lossy().to_string();
                    Some(Mib::from_str(&s))
                } else {
                    // TODO: hanlde symlink
                    unimplemented!()
                }
            },
            Some(Err(e)) => return Some(Err(e)),
            None => {
                self.dirs.remove(idx);
                self.next()
            }
        }
    }
}
