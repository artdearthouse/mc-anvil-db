use fuser::{FileAttr, FileType, Filesystem, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

pub struct McFUSE;



const DIR_ATTR_TEMPLATE: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 0, gid: 0, rdev: 0, blksize: 512, flags: 0, // uid/gid 0 is ok, we will handle it in code for portability
};

const FILE_ATTR_TEMPLATE: FileAttr = FileAttr {
    ino: 2,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 0, gid: 0, rdev: 0, blksize: 512, flags: 0,
};

impl Filesystem for McFUSE {
    // 1. GETATTR (File attributes)
    fn getattr(&mut self, req: &Request, ino: u64, _fh: Option<u64>, reply: fuser::ReplyAttr) {
        match ino {
            1 => { // Directory
                let mut attr = DIR_ATTR_TEMPLATE;
                attr.uid = req.uid(); attr.gid = req.gid();
                reply.attr(&Duration::from_secs(1), &attr);
            },
            2 => { // Our file r.0.0.mca
                let mut attr = FILE_ATTR_TEMPLATE;
                attr.uid = req.uid(); attr.gid = req.gid();
                reply.attr(&Duration::from_secs(1), &attr);
            },
            _ => reply.error(ENOENT),
        }
    }

    // 2. LOOKUP (Name search: "What is the inode for r.0.0.mca?")
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
        if parent == 1 && name.to_str() == Some("r.0.0.mca") {
            let mut attr = FILE_ATTR_TEMPLATE;
            attr.uid = req.uid(); attr.gid = req.gid();
            // Generation = 0 (file version), TTL = 1 sec
            reply.entry(&Duration::from_secs(1), &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    // 3. READDIR (LS: "What is inside the folder?")
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: fuser::ReplyDirectory) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        // offset - is the cursor. FUSE can read the directory in chunks.
        // We return: (inode, type, name).
        // Important: offset increases by 1 for each subsequent entry.
        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "r.0.0.mca"),
        ];

        for (i, entry) in entries.into_iter().enumerate() {
            // i + 1, because offset 0 implies "start", and the next entry will be 1, 2, 3...
            if i as i64 >= offset {
                // add returns true if the buffer is full.
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
        }
        reply.ok();
    }

    // 4. WRITE (Write into void)
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, _offset: i64, data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: Option<u64>, reply: fuser::ReplyWrite) {
        if ino == 2 {
            // "Honestly" say that we wrote as many bytes as sent
            println!("Writing {} dummy bytes to inode {}", data.len(), ino);
            reply.written(data.len() as u32);
        } else {
            reply.error(ENOENT);
        }
    }
}