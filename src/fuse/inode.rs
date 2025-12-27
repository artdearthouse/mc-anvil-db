
// 30 bits for X and Z. Range +/- 500 million.
// Flag at bit 63 (0x8000...)
// Structure: [1: Flag] [1: Unused] [30: X] [2: Unused] [30: Z]
// Actually simpler: 
// [1: Flag] [1: Unused] [31: X(30 bits?? no, let's use 30)] -> 32 + 30 = 62.
// Let's do:
// Bit 63: Flag
// Bit 32..61: X (30 bits)
// Bit 0..29: Z (30 bits)
// 30 bits gives 1 billion range. Offset by 500,000,000.

const OFFSET: i32 = 500_000_000;
const MASK: u64 = 0x3FFFFFFF; // 30 bits

pub const REGION_INODE_START: u64 = 0x8000_0000_0000_0000;
pub const GENERIC_INODE_START: u64 = 0x4000_0000_0000_0000;

pub fn is_region_inode(ino: u64) -> bool {
    (ino & REGION_INODE_START) != 0
}

pub fn is_generic_inode(ino: u64) -> bool {
    (ino & GENERIC_INODE_START) != 0
}

pub fn pack(x: i32, z: i32) -> u64 {
    // We assume x and z are within +/- 500M. Minecraft is +/- 60k regions. Safe.
    let x_enc = (x + OFFSET) as u64 & MASK;
    let z_enc = (z + OFFSET) as u64 & MASK;
    
    REGION_INODE_START | (x_enc << 32) | z_enc
}

// FNV-1a 64-bit hash
fn fnv1a_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x1099511628211);
    }
    hash
}

pub fn pack_generic(name: &str) -> u64 {
    let hash = fnv1a_hash(name);
    // Mask to 62 bits to avoid colliding with flags (top 2 bits)
    // Actually we just set the second highest bit
    GENERIC_INODE_START | (hash & 0x3FFF_FFFF_FFFF_FFFF)
}

pub fn unpack(ino: u64) -> Option<(i32, i32)> {
    if !is_region_inode(ino) {
        return None;
    }
    
    let x_enc = (ino >> 32) & MASK;
    let z_enc = ino & MASK;
    
    let x = (x_enc as i32) - OFFSET;
    let z = (z_enc as i32) - OFFSET;
    
    Some((x, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack() {
        let coords = [
            (0, 0),
            (1, 1),
            (-1, -1),
            (100, -100),
            (10_000_000, -10_000_000), // Minecraft world limit is ~30M blocks (~60k regions), this is plenty
            // (i32::MAX, i32::MIN), // We don't support full i32 anymore, only +/- 500M
            (499_999_999, -499_999_999), 
        ];

        for (x, z) in coords {
            let ino = pack(x, z);
            assert!(is_region_inode(ino));
            let (rx, rz) = unpack(ino).expect("Should unpack");
            assert_eq!(rx, x);
            assert_eq!(rz, z);
        }
    }

    #[test]
    fn test_generic_inodes() {
        let name = "backup.mca";
        let ino = pack_generic(name);
        assert!(is_generic_inode(ino));
        assert!(!is_region_inode(ino));
        
        // Hash stability check (FNV-1a of "backup.mca" should be stable)
        let ino2 = pack_generic(name);
        assert_eq!(ino, ino2);
        
        let name2 = "other.file";
        let ino3 = pack_generic(name2);
        assert_ne!(ino, ino3);
    }
    #[test]
    fn test_system_inode() {
        assert!(!is_region_inode(1));
        assert!(!is_region_inode(2));
        assert_eq!(unpack(1), None);
    }
}
