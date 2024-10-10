use kernel::prelude::*; // General kernel module utilities
use kernel::file_operations; // Equivalent to <linux/fs.h>
use kernel::spinlock::SpinLock; // Equivalent to <linux/spinlock.h>
use kernel::sync::Mutex; // Equivalent to <linux/mutex.h>
use kernel::kdev_t; // Equivalent to <linux/kdev_t.h>
use kernel::blkdev; // Block device management like <linux/blkdev.h>
// Additional crate imports to handle kernel structures and functions

// TODO: Implement equivalent of other features like block tracing, proc_fs, and more
// if they exist in the Rust ecosystem for kernel development.

use core::sync::atomic::{AtomicU64, Ordering};  // For atomic operations
use kernel::kobject::KObject;                  // Hypothetical Rust equivalent for `kobject`

// Static kernel object placeholder (will need proper initialization as in the C code)
static mut BLOCK_DEPR: Option<KObject> = None;

// Unique, monotonically increasing sequential number for block devices
static DISKSEQ: AtomicU64 = AtomicU64::new(0);

// Define constants using bit shifting
const NR_EXT_DEVT: u32 = 1 << MINORBITS;  // Assume MINORBITS is predefined

use kernel::ida::Ida;  // Hypothetical IDA equivalent in Rust for kernel

// Static global IDA for managing external device type IDs
static mut EXT_DEVT_IDA: Option<Ida> = None;

use kernel::prelude::*;  // Kernel macros and traits
use kernel::blkdev::Gendisk;  // Hypothetical Rust equivalent of `gendisk`
use kernel::blkdev::bdev_set_nr_sectors;  // Hypothetical function to set number of sectors

/// Set the capacity of a block device in terms of sectors.
pub fn set_capacity(disk: &mut Gendisk, sectors: u64) {
    bdev_set_nr_sectors(&mut disk.part0, sectors);
}

// Export the symbol for other kernel modules
module_export!(set_capacity);

use kernel::prelude::*;
use kernel::blkdev::{Gendisk, get_capacity, set_capacity};  // Hypothetical Rust equivalents
use kernel::kobject::{KObject, kobject_uevent_env};
use kernel::log::info;  // For kernel logging

// Define a flag for GENHD_FL_HIDDEN, if not already present
const GENHD_FL_HIDDEN: u32 = 0x10; // Placeholder value

/// Set capacity and notify if necessary.
pub fn set_capacity_and_notify(disk: &mut Gendisk, size: u64) -> bool {
    let capacity = get_capacity(disk);  // Get current disk capacity
    let envp: [&str; 2] = ["RESIZE=1", "\0"];  // Environment parameter for uevent

    set_capacity(disk, size);  // Set new capacity

    // Check conditions to avoid spamming logs/uevents
    if size == capacity || !disk_live(disk) || (disk.flags & GENHD_FL_HIDDEN != 0) {
        return false;
    }

    // Log the capacity change if it's user visible and alive
    info!(
        "{}: detected capacity change from {} to {}",
        disk.disk_name, capacity, size
    );

    // Avoid sending uevent for changes to/from an empty device
    if capacity == 0 || size == 0 {
        return false;
    }

    // Trigger a uevent to notify userspace
    kobject_uevent_env(&disk_to_dev(disk).kobj, kernel::uevent::KOBJ_CHANGE, &envp);
    
    true
}

// Export the symbol for GPL modules
module_export_gpl!(set_capacity_and_notify);

use kernel::prelude::*;
use kernel::sync::percpu::PerCpu;  // Hypothetical per-CPU abstraction
use core::mem::MaybeUninit;  // For zeroing memory safely

// Structure definitions for block device and disk statistics
struct BlockDevice {
    bd_stats: PerCpu<DiskStats>,  // Hypothetical per-CPU stats field
}

struct DiskStats {
    nsecs: [u64; NR_STAT_GROUPS],
    sectors: [u64; NR_STAT_GROUPS],
    ios: [u64; NR_STAT_GROUPS],
    merges: [u64; NR_STAT_GROUPS],
    io_ticks: u64,
}

// Function to aggregate statistics from all CPUs
pub fn part_stat_read_all(part: &BlockDevice, stat: &mut DiskStats) {
    // Zero out the stat structure
    *stat = Default::default();

    // Iterate over all possible CPUs and aggregate stats
    for cpu in kernel::cpu::possible_cpus() {
        let ptr = part.bd_stats.get(cpu);  // Get per-CPU stats pointer

        // Aggregate stats for all groups
        for group in 0..NR_STAT_GROUPS {
            stat.nsecs[group] += ptr.nsecs[group];
            stat.sectors[group] += ptr.sectors[group];
            stat.ios[group] += ptr.ios[group];
            stat.merges[group] += ptr.merges[group];
        }

        stat.io_ticks += ptr.io_ticks;
    }
}

use kernel::prelude::*;
use kernel::cpu::possible_cpus;  // Hypothetical function to get possible CPUs
use kernel::blkdev::part_stat_local_read_cpu;  // Hypothetical per-CPU stat reader

/// Get the number of in-flight I/O requests for the block device.
pub fn part_in_flight(part: &BlockDevice) -> u32 {
    let mut inflight: u32 = 0;

    // Iterate over all possible CPUs
    for cpu in possible_cpus() {
        inflight += part_stat_local_read_cpu(part, 0, cpu) + 
                    part_stat_local_read_cpu(part, 1, cpu);
    }

    // Ensure the result is non-negative (though unsigned types can't be negative)
    // Rust's unsigned integers handle underflows safely, so no need to check negative explicitly
    inflight
}

use kernel::prelude::*;
use kernel::cpu::possible_cpus;  // Hypothetical function to iterate over all CPUs
use kernel::blkdev::part_stat_local_read_cpu;  // Hypothetical per-CPU stat reader

/// Reads and sums the in-flight read and write requests for a block device.
pub fn part_in_flight_rw(part: &BlockDevice, inflight: &mut [u32; 2]) {
    // Initialize both read and write counters to 0
    inflight[0] = 0;
    inflight[1] = 0;

    // Iterate over all possible CPUs and aggregate the stats
    for cpu in possible_cpus() {
        inflight[0] += part_stat_local_read_cpu(part, 0, cpu);  // Read stats
        inflight[1] += part_stat_local_read_cpu(part, 1, cpu);  // Write stats
    }

    // No need to check for negative values since `u32` is always non-negative
    // (Rust's type system ensures this)
}

use kernel::prelude::*;
use kernel::sync::{Mutex, SpinLock};  // For kernel mutex and spinlock

// Define the size of the major hash table
const BLKDEV_MAJOR_HASH_SIZE: usize = 255;

// Structure representing major block device names
struct BlkMajorName {
    next: Option<Box<BlkMajorName>>,  // Linked list pointer to the next entry
    major: i32,                       // Major number
    name: [u8; 16],                   // Name (16-byte array to store the string)
    #[cfg(CONFIG_BLOCK_LEGACY_AUTOLOAD)]
    probe: Option<fn(dev_t)>,          // Optional probe function (if the config is enabled)
}

// Array of pointers to BlkMajorName structures (hash table)
static mut MAJOR_NAMES: [Option<Box<BlkMajorName>>; BLKDEV_MAJOR_HASH_SIZE] = [None; BLKDEV_MAJOR_HASH_SIZE];

// Mutex to protect access to `major_names`
static MAJOR_NAMES_LOCK: Mutex<()> = Mutex::new(());

// Spinlock for finer synchronization around `major_names`
static MAJOR_NAMES_SPINLOCK: SpinLock<()> = SpinLock::new(());

// Define a function to map a major number to an index in the hash table
#[inline]
fn major_to_index(major: u32) -> usize {
    (major as usize) % BLKDEV_MAJOR_HASH_SIZE
}

