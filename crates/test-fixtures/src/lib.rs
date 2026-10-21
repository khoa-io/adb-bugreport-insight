//! Shared ZIP fixtures for the `mcp` and `cli` test suites.
//!
//! **Why there is exactly one copy.** Two hand-rolled writers drifted: the
//! central-directory header is 46 bytes before the name, and a copy that writes
//! 44 produces an archive `zip` rejects with `InvalidArchive` — which surfaces as
//! `NotAZipArchive`, not as a clear test failure. Keeping one copy means a field
//! cannot be missed in only one suite.
//!
//! **Why this is a crate rather than a `#[path]` module.** Both suites need it
//! and they live in different packages. A `#[path = "../../mcp/…"]` module
//! resolves under `rustc` but not under rust-analyzer, which reports it as an
//! unresolved module and then loses every fixture behind it. A `dev-dependency`
//! resolves identically in both tools and is checked by cargo itself.
//!
//! Reachable from test builds only: this is a dev-dependency of `crates/mcp` and
//! `crates/cli` and of nothing else.

use std::path::PathBuf;

pub fn scratch_dir() -> PathBuf {
    // This crate sits at `crates/<name>/`, so two levels up is the workspace
    // root and `target/` is shared with every other suite.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-fixtures");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Minimal **stored**-entry ZIP writer.
///
/// Stored keeps entries byte-exact, so a test can corrupt one at a known offset
/// and produce "a valid central directory over bytes that do not decode".
pub struct ZipBuilder {
    buf: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            central: Vec::new(),
            count: 0,
        }
    }

    pub fn add(mut self, name: &str, data: &[u8]) -> Self {
        let crc = crc32(data);
        let offset = self.buf.len() as u32;

        // Local file header: 30 bytes, then name and payload.
        self.buf.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // flags
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // mod time
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // mod date
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed
        self.buf
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // extra length
        self.buf.extend_from_slice(name.as_bytes());
        self.buf.extend_from_slice(data);

        // Central directory header: 46 bytes, then the name.
        self.central
            .extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        self.central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        self.central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.central.extend_from_slice(&0u16.to_le_bytes()); // flags
        self.central.extend_from_slice(&0u16.to_le_bytes()); // method
        self.central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        self.central.extend_from_slice(&0u16.to_le_bytes()); // mod date
        self.central.extend_from_slice(&crc.to_le_bytes());
        self.central
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.central
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.central
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.central.extend_from_slice(&0u16.to_le_bytes()); // extra length
        self.central.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        self.central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        self.central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        self.central.extend_from_slice(&offset.to_le_bytes());
        self.central.extend_from_slice(name.as_bytes());

        self.count += 1;
        self
    }

    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buf.len() as u32;
        let cd_size = self.central.len() as u32;
        self.buf.extend_from_slice(&self.central);
        self.buf.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // disk number
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&cd_size.to_le_bytes());
        self.buf.extend_from_slice(&cd_offset.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.buf
    }
}

/// A normal archive: three entries, one of them containing a known pattern.
///
/// Built **once** per process. Cargo runs tests in parallel threads and several
/// tests call this; writing a shared file each time was a race that surfaced as
/// `NotAZipArchive` in whichever test lost.
pub fn good_archive() -> PathBuf {
    static GOOD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    GOOD.get_or_init(|| {
        let path = scratch_dir().join("good.zip");
        let bytes = ZipBuilder::new()
            .add("version.txt", b"2\n")
            .add("ANR/traces.txt", b"FATAL EXCEPTION: main\nsecond line\n")
            .add("ANR/other.txt", b"nothing here\n")
            .finish();
        std::fs::write(&path, bytes).expect("write good fixture");
        path
    })
    .clone()
}

/// The same archive with one entry's **stored bytes** altered so its CRC no
/// longer matches: a valid central directory over a corrupt payload. This is
/// the fixture that catches an implementation returning `empty` for a read it
/// could not perform.
pub fn corrupt_entry_archive() -> PathBuf {
    static CORRUPT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CORRUPT
        .get_or_init(|| {
            let mut bytes = std::fs::read(good_archive()).expect("read good fixture");
            let needle = b"FATAL EXCEPTION: main\n";
            let pos = bytes
                .windows(needle.len())
                .position(|w| w == needle)
                .expect("fixture must contain the payload");
            bytes[pos] = b'X';
            let path = scratch_dir().join("corrupt.zip");
            std::fs::write(&path, bytes).expect("write corrupt fixture");
            path
        })
        .clone()
}

/// A file that is not a ZIP at all.
pub fn not_a_zip() -> PathBuf {
    static NOT_ZIP: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    NOT_ZIP
        .get_or_init(|| {
            let path = scratch_dir().join("not-a-zip.bin");
            std::fs::write(&path, b"this is definitely not a zip archive")
                .expect("write non-zip fixture");
            path
        })
        .clone()
}

/// Synthetic Android 15 report exercising every insight domain and clock family.
pub fn insight_archive() -> PathBuf {
    static INSIGHT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    INSIGHT.get_or_init(|| {
        let mut main = String::from("== dumpstate: 2026-10-17 12:00:00+00:00\nBuild: test-user 15 TEST 1 release-keys\nBuild fingerprint: test/product/device:15/TEST/1:user/release-keys\nAndroid SDK version: 35\nKernel: Linux version 6.6.0\nCommand line: console=ttyS0\nUptime: up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: 1.00, 0.50, 0.25\n------ SYSTEM PROPERTIES (getprop) ------\n[ro.product.name]: [product]\n[ro.product.device]: [device]\n[ro.product.brand]: [test]\n[ro.build.type]: [user]\n[ro.build.version.sdk]: [35]\n[ro.hardware]: [ranchu]\n------ 0.001s was the duration of 'SYSTEM PROPERTIES' ------\n------ SYSTEM LOG (logcat) ------\n10-17 11:59:55.000 root 1 1 I init: first retained line\n");
        for second in 0..5 {
            main.push_str(&format!("10-17 12:00:0{second}.100 1010001 {} {} E AndroidRuntime: FATAL EXCEPTION: main\n10-17 12:00:0{second}.100 1010001 {} {} E AndroidRuntime: Process: pkg.app, PID: {}\n10-17 12:00:0{second}.100 1010001 {} {} E AndroidRuntime: java.lang.IllegalStateException: synthetic\n10-17 12:00:0{second}.100 1010001 {} {} E AndroidRuntime:     at pkg.app.Main.run(Main.java:12)\n", 100+second,100+second,100+second,100+second,100+second,100+second,100+second,100+second,100+second));
        }
        main.push_str("10-17 12:00:05.000 system 10 11 W ActivityManager: Process pkg.app has crashed too many times: crashed quickly\n10-17 12:00:06.000 system 10 11 E ActivityManager: ANR in pkg.other\n10-17 12:00:06.000 system 10 11 E ActivityManager: PID: 55\n10-17 12:00:06.000 system 10 11 E ActivityManager: Reason: Input dispatching timed out, Waited 5010ms\n10-17 12:00:08.000 system 42 43 W pkg.app: Long monitor contention with owner Worker (44) at void pkg.Lock.run()(Lock.java:9) waiters=2 in void pkg.Lock.waitNow() for 1.500s\n10-17 12:00:10.000 root 1 1 I init: last retained line\n------ EVENT LOG (logcat -b events) ------\n10-17 11:59:59.000 system 10 11 I am_proc_start: [10,42,1010001,pkg.app,service,pkg.app/.Service]\n10-17 12:00:00.000 system 10 11 I am_proc_bound: [10,42,pkg.app]\n10-17 12:00:06.010 system 10 11 I am_anr: [10,55,pkg.other,1,Input dispatching timed out]\n10-17 12:00:07.000 system 10 11 I dvm_lock_sample: [system_server,1,binder,100,Lock.java,9,waitNow,-,10,run,50]\n------ CPU INFO (top -b -n 1 -H) ------\nThreads: 10 total, 1 running, 9 sleeping, 0 stopped, 0 zombie\n200%cpu 80%user 10%nice 50%sys 40%idle 0%iow 10%irq 10%sirq 0%host\nSwap: 1000K total, 250K used, 750K free\n------ MEMORY INFO (/proc/meminfo) ------\nMemTotal: 10000 kB\nMemFree: 2000 kB\nSwapTotal: 1000 kB\nSwapFree: 900 kB\nSlab: 100 kB\nPageTables: 20 kB\nKernelStack: 10 kB\nCmaTotal: 0 kB\n------ VM STAT (cat /proc/vmstat) ------\noom_kill 0\npswpin 1\npswpout 2\n------ PROCRANK ------\nPID Vss Rss Pss Uss Swap PSwap USwap ZSwap cmdline\n42 100K 90K 50K 20K 10K 8K 7K 2K pkg.app\nZRAM: 100K physical used for 200K in swap (1000K total swap)\n------ 0.100s was the duration of 'PROCRANK' ------\n------ DUMPSYS (/system/bin/dumpsys) ------\n10-17 12:00:00.100 system 999 999 E AndroidRuntime: FATAL EXCEPTION: main\n10-17 12:00:00.100 system 999 999 E AndroidRuntime: Process: pkg.app, PID: 100\n10-17 12:00:00.100 system 999 999 E AndroidRuntime: java.lang.IllegalStateException: synthetic\n10-17 12:00:00.100 system 999 999 E AndroidRuntime:     at pkg.app.Main.run(Main.java:12)\nDUMP OF SERVICE HIGH cpuinfo:\nCPU usage from 3000ms to 1000ms ago (2026-10-17 11:59:57.000 to 2026-10-17 11:59:59.000):\n  50% 42/pkg.app: 30% user + 20% kernel / faults: 2 minor 1 major\n85% TOTAL: 40% user + 30% kernel + 5% iowait + 5% irq + 5% softirq\n--------- 0.001s was the duration of dumpsys cpuinfo, ending at: 2026-10-17 12:00:02\nDUMP OF SERVICE cpu_monitor:\nmLastReadUptimeMillis = 32000\nCpuInfo{ cpuCore = 0, isOnline = Yes, latestCpuUsageStats = CpuUsageStats{ userTimeMillis = 99999 } }\nLatest cumulative CPU usage stats by CPU core:\n CPU core id = 0, CpuUsageStats{ userTimeMillis = 100, niceTimeMillis = 20, systemTimeMillis = 80, idleTimeMillis = 500, iowaitTimeMillis = 100, irqTimeMillis = 50, softirqTimeMillis = 50, stealTimeMillis = 100, guestTimeMillis = 10, guestNiceTimeMillis = 5 }\n--------- 0.001s was the duration of dumpsys cpu_monitor, ending at: 2026-10-17 12:00:02\nDUMP OF SERVICE HIGH meminfo:\nUptime: 40000 Realtime: 40000\nTotal PSS by process:\n  100K: pkg.app (pid 42) (10K in swap)\nTotal PSS by category:\n  70K: Native (5K in swap)\n  30K: Dalvik (5K in swap)\nTotal RAM: 10000K (status normal)\nFree RAM: 4000K ( 1000K cached pss + 1000K cached kernel + 2000K free)\nUsed RAM: 5000K (3000K used pss + 2000K kernel)\nLost RAM: 1000K\nZRAM: 150K physical used for 300K in swap (1000K total swap)\nTuning: 192 (large 576), oom 200K, restore limit 100K (high-end-gfx)\n--------- 1.000s was the duration of dumpsys meminfo, ending at: 2026-10-17 12:00:10\nDUMP OF SERVICE package:\nPackages:\n Package [pkg.app] (abc):\n  userId=10001\n  codePath=/system/app/Test\n  resourcePath=/system/app/Test\n  versionCode=1 minSdk=30 targetSdk=35\n  versionName=1.0\n  flags=[ SYSTEM HAS_CODE ]\n  privateFlags=[ PRIVILEGED ]\n  User 0: ceDataInode=0 deDataInode=0 installed=false stopped=true notLaunched=true hidden=false suspended=false quarantined=false enabled=0\n   firstInstallTime=1970-01-01 00:00:00\n  User 10: ceDataInode=123 deDataInode=456 installed=true stopped=false notLaunched=false hidden=false suspended=false quarantined=false enabled=0\n   firstInstallTime=2026-10-01 00:00:00\n--------- 0.001s was the duration of dumpsys package, ending at: 2026-10-17 12:00:07\nDUMP OF SERVICE activity:\n *APP* UID 1010001 ProcessRecord{abc 42:pkg.app/u10a1}\n  packageList={pkg.app}\n  startSeq=7\n  startUpTime=-8s\n  codePath=/system/app/Test\n--------- 0.001s was the duration of dumpsys activity, ending at: 2026-10-17 12:00:07\nDUMP OF SERVICE user:\nCurrent user: 10\nUserInfo{10:Driver:412} serialNo=10\n Flags: ADMIN|FULL|INITIALIZED\n Type: android.os.usertype.full.SECONDARY\n State: RUNNING_UNLOCKED\n--------- 0.001s was the duration of dumpsys user, ending at: 2026-10-17 12:00:07\nDUMP OF SERVICE procstats:\nCURRENT STATS:\nStart time: 2026-10-17 11:59:55\nTotal uptime: +15s\nTotal elapsed time: +15s (partial)\nAggregated over: 1\nAGGREGATED OVER LAST 24 HOURS:\nStart time: 2026-05-01 00:00:00\nAggregated over: 10\nAGGREGATED OVER LAST 3 HOURS:\nStart time: 2026-05-01 00:00:00\nAggregated over: 10\n--------- 0.001s was the duration of dumpsys procstats, ending at: 2026-10-17 12:00:10\n");
        let trace=b"----- dumping pid: 55 at 1000000 -----\n----- pid 55 at 2026-05-01 00:00:00.000000000+0000 -----\nCmd line: pkg.other\nDALVIK THREADS (2):\n- waiting to lock <abc> held by thread 2\n";
        let path=scratch_dir().join("insight-complete.zip");
        let bytes=ZipBuilder::new().add("version.txt",b"2.0").add("main_entry.txt",b"bugreport.txt").add("bugreport.txt",main.as_bytes())
            .add("FS/data/anr/anr_2026-05-01-00-00-00-000",trace)
            .add("FS/data/tombstones/tombstone_00",b"*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***\nTimestamp: 2026-05-01 00:00:00.000000000+0000\nCmdline: pkg.native\npid: 88, tid: 89, name: worker  >>> pkg.native <<<\nsignal 6 (SIGABRT), code -1 (SI_QUEUE), fault addr --------\nAbort message: 'synthetic failure'\n    #00 pc 00000000 /apex/libc.so (abort+1)\n    #01 pc 00000010 /vendor/libroot.so (rootCause+4)\n")
            .finish();std::fs::write(&path,bytes).expect("write synthetic insight fixture");path
    }).clone()
}

/// Marks selected entries encrypted without encrypting their synthetic payloads.
/// Readers must refuse these entries before consuming their bytes.
pub fn mark_encrypted(mut bytes: Vec<u8>, entry_ids: &[usize]) -> Vec<u8> {
    let end = bytes.len() - 22;
    let mut central = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap()) as usize;
    for id in 0..count {
        assert_eq!(&bytes[central..central + 4], b"PK\x01\x02");
        if entry_ids.contains(&id) {
            let local =
                u32::from_le_bytes(bytes[central + 42..central + 46].try_into().unwrap()) as usize;
            bytes[local + 6] |= 1;
            bytes[central + 8] |= 1;
        }
        let lengths = [28, 30, 32].map(|off| {
            u16::from_le_bytes(bytes[central + off..central + off + 2].try_into().unwrap()) as usize
        });
        central += 46 + lengths.iter().sum::<usize>();
    }
    bytes
}
