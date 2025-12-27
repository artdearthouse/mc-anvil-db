//! FUSE filesystem implementation for Minecraft region files.
//!
//! Provides a virtual filesystem that serves procedurally generated
//! Minecraft world data in the Anvil format.

mod inode;

use std::sync::Arc;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;

use crate::chunk::ChunkProvider;
use crate::region::{self, Header, RegionPos, SECTOR_SIZE, HEADER_SIZE};
use crate::storage::ChunkStorage;

use inode::InodeMap;

/// TTL for cached file attributes.
const TTL: Duration = Duration::from_secs(1);

/// Virtual file size for region files.
/// 1024 chunks * 128KB (32 sectors) + 8KB header = ~134 MB.
/// We use a safe upper bound.
const VIRTUAL_FILE_SIZE: u64 = 1024 * 32 * 4096 + 8192;

/// FUSE filesystem for Minecraft Anvil regions.
pub struct AnvilFS {
    inodes: InodeMap,
    chunks: ChunkProvider,
}

impl AnvilFS {
    pub fn new(storage: Arc<dyn ChunkStorage>) -> Self {
        Self {
            inodes: InodeMap::new(),
            chunks: ChunkProvider::new(storage),
        }
    }

    fn root_attr() -> FileAttr {
        FileAttr {
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
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn file_attr(ino: u64) -> FileAttr {
        FileAttr {
            ino,
            size: VIRTUAL_FILE_SIZE,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    /// Read data from a virtual region file.
    /// Writes directly to the `buf` slice.
    fn read_region(&self, region: RegionPos, offset: usize, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = buf.len();
        let end = offset + size;

        // Fetch present chunks from storage (one query per read request is robust, though cacheable)
        // Optimization: For now we query every time.
        let present_coords = self.chunks.get_storage().get_region_chunks(region);
        let present_indices: Vec<usize> = present_coords.iter()
            .map(|p| {
                 let lx = p.x - region.x * 32;
                 let lz = p.z - region.z * 32;
                 region::local_to_index(lx, lz)
            })
            .collect();
            
        // Debug
        // log::info!("Reading region {:?}. Present chunks: {}", region, present_indices.len());

        // Zone A: Header (0 - HEADER_SIZE)
        if offset < HEADER_SIZE {
            let header = Header::get_range(&present_indices, offset, size);
            let copy_len = std::cmp::min(header.len(), buf.len());
            buf[..copy_len].copy_from_slice(&header[..copy_len]);
        }

        // Zone B: Chunk data (HEADER_SIZE+)
        if end > HEADER_SIZE {
            let chunk_size = SECTOR_SIZE * crate::region::CHUNK_STRIDE as usize;
            
            let data_start = std::cmp::max(offset, HEADER_SIZE);
            let first_chunk = (data_start - HEADER_SIZE) / chunk_size;
            let last_chunk = (end - HEADER_SIZE - 1) / chunk_size;

            for chunk_idx in first_chunk..=last_chunk {
                if chunk_idx >= 1024 {
                    break;
                }
                
                // Skip if not present in our list
                if !present_indices.contains(&chunk_idx) {
                    continue; // Leave buffer as zeros (empty)
                }

                // Virtual file layout based on fixed stride
                let chunk_sector_start = 2 + chunk_idx as u32 * region::CHUNK_STRIDE;
                let chunk_sector_count = region::CHUNK_STRIDE; // 128KB max

                let chunk_file_start = (chunk_sector_start as usize) * SECTOR_SIZE;
                let chunk_file_end = chunk_file_start + (chunk_sector_count as usize) * SECTOR_SIZE;

                let overlap_start = std::cmp::max(offset, chunk_file_start);
                let overlap_end = std::cmp::min(end, chunk_file_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                // Get chunk world coordinates
                let (local_x, local_z) = region::index_to_local(chunk_idx);
                let (world_x, world_z) = region.local_to_world(local_x, local_z);

                // Get chunk data (from storage)
                let pos = crate::storage::ChunkPos::new(world_x, world_z);
                
                // Only get if it exists
                if let Ok(blob) = self.chunks.get_chunk(pos) {
                   if blob.is_empty() { continue; }

                    // Copy relevant portion
                    let blob_start = overlap_start - chunk_file_start;
                    let blob_end = overlap_end - chunk_file_start;
                    let result_start = overlap_start - offset;

                    for i in blob_start..blob_end {
                        let result_idx = result_start + (i - blob_start);
                        if result_idx < size && i < blob.len() {
                            buf[result_idx] = blob[i];
                        }
                    }
                }
            }
        }

        Ok(size)
    }
}

impl Filesystem for AnvilFS {
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        if ino == 1 {
            reply.attr(&TTL, &Self::root_attr());
        } else {
            reply.attr(&TTL, &Self::file_attr(ino));
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let filename = name.to_str().unwrap_or("");

        if let Some(region) = RegionPos::from_filename(filename) {
            let ino = self.inodes.get_or_create(region);
            reply.entry(&TTL, &Self::file_attr(ino), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino == 1 {
            if offset == 0 {
                let _ = reply.add(1, 0, FileType::Directory, ".");
                let _ = reply.add(1, 1, FileType::Directory, "..");
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if let Some(region) = self.inodes.get(ino) {
            let mut data = vec![0u8; size as usize];
            match self.read_region(region, offset as usize, &mut data) {
                Ok(_) => reply.data(&data),
                Err(e) => {
                    log::error!("Failed to read region {:?}: {}", region, e);
                    reply.error(libc::EIO);
                }
            }
        } else {
            reply.data(&[0u8; 0]);
        }
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        reply.opened(0, 0);
    }

    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        log::info!("FUSE WRITE: offset={}, len={}", offset, data.len());

        // We only care about chunk data writes, not header updates.
        // Chunk data usually starts after the standard header (8192 bytes).
        if offset >= 8192 && data.len() > 5 {
             match self.chunks.save_chunk(data) {
                Ok(_) => {
                    log::info!("Chunk save successful");
                    reply.written(data.len() as u32);
                },
                Err(e) => {
                    log::warn!("Failed to save chunk at offset {}: {}", offset, e);
                    reply.written(data.len() as u32);
                }
             }
        } else {
            if offset < 8192 {
                log::info!("Ignoring header write at {}", offset);
            } else {
                log::info!("Ignoring small/invalid write at {}, len={}", offset, data.len());
            }
            // Ignore header writes or tiny fragments
            reply.written(data.len() as u32);
        }
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn flush(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn mknod(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _u: u32,
        _rdev: u32,
        reply: fuser::ReplyEntry,
    ) {
        reply.error(libc::EPERM);
    }

    fn create(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _u: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        reply.error(libc::EPERM);
    }

    fn unlink(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: fuser::ReplyEmpty) {
        reply.error(libc::EPERM);
    }

    fn rename(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _newparent: u64,
        _newname: &OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        reply.error(libc::EPERM);
    }
}
