use std::fmt;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::{env, time};
use walkdir::{DirEntry, WalkDir};

#[allow(dead_code)]
fn main() {
    let start = time::Instant::now();
    let args: Vec<String> = env::args().collect();

    let root = parse_path(&args); // Determine path
    let dir = WalkDir::new(&root);

    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = {
        let pool = pool(dir); // Retrieve entries from WalkDir
        partition_from(pool) // Partition files from directories
    };

    let (fs_count, dr_count) = (files.len(), dirs.len());
    let (file_counter, total_size, file_ranking) = file_count(files);

    {
        println!("++ File size distribution for : {} ++\n", &root.display());
        println!("Files @ 0B            : {:4}", file_counter[0]);
        println!("Files > 1B  - 1,023B  : {:4}", file_counter[1]);
        println!("Files > 1KB - 1,023KB : {:4}", file_counter[2]);
        println!("Files > 1MB - 1,023MB : {:4}", file_counter[3]);
        println!("Files > 1GB - 1,023GB : {:4}", file_counter[4]);
        println!("Files > 1TB+          : {:4}\n", file_counter[5]);

        println!("Files encountered: {}", fs_count);
        println!("Directories traversed: {}", dr_count);
        println!(
            "Total size of all files: {}\n",
            Filesize::<Kilobytes>::from(total_size)
        );

        println!("Largest files:");
        for (fsize, fpath, fname) in file_ranking {
            println!("    Filesize: {:?} B", fsize);
            println!("    Filepath: {:?}", fpath);
            println!("    Filename: {:?}\n", fname);
        };
    }

    let end = time::Instant::now();
    println!("Run time: {:?}\n", end.duration_since(start));
}

fn parse_path(args: &[String]) -> &Path {
    // If there's no `args` entered, the executable will search it's own path.
    match args.len() {
        1 => Path::new(&args[0]),
        _ => Path::new(&args[1]),
    }
}

fn pool(dir: WalkDir) -> Vec<DirEntry> {
    // Check each item for errors and drop possible invalid `DirEntry`s
    dir.into_iter().filter_map(|e| e.ok()).collect()
}

fn partition_from(pool: Vec<DirEntry>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // Read `Path` from `DirEntry`, checking if `Path` is a file or directory.
    pool.into_iter()
        .map(|e| e.into_path())
        .partition(|path| path.is_file())
}

fn file_count(files: Vec<PathBuf>) -> ([u64; 6], u64, Vec<(u64, PathBuf, String)>) {
    let mut file_ranking: Vec<(u64, PathBuf, String)> = vec![];
    let mut counter: [u64; 6] = [0; 6];

    for file in &files {
        // Record filesizes
        let fsize: u64 = Filesize::<Bytes>::from(file).bytes;
        let fpath: PathBuf = file.as_path().to_path_buf();
        let fname: String = file
            .file_name()
            .expect("could not transform to OsStr")
            .to_os_string()
            .into_string()
            .expect("could not release String from Option");

        match fsize {
            0 => counter[0] += 1,                                 // Empty file
            1..=1_023 => counter[1] += 1,                         // 1 byte to 0.99KB
            1_024..=1_048_575 => counter[2] += 1,                 // 1 kilo to 0.99MB
            1_048_576..=1_073_741_823 => counter[3] += 1,         // 1 mega to 0.99GB
            1_073_741_824..=1_099_511_627_775 => counter[4] += 1, // 1 giga to 0.99TB
            1_099_511_627_776..=std::u64::MAX => counter[5] += 1, // 1 terabyte or larger
        };

        file_ranking.push((fsize, fpath, fname));
        file_ranking.sort_unstable();
    }

    file_ranking.reverse(); // Pull largest elements into first 10 indices
    file_ranking.truncate(10); // Cut off remainder of 11+ elements

    let total_file_size: &u64 = &files
        .iter()
        .fold(0, |acc, file| acc + Filesize::<Bytes>::from(file).bytes);

    (counter, *total_file_size, file_ranking)
}

trait SizeUnit: Copy {
    fn singular_name() -> String;
    fn num_byte_in_unit() -> u64;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Bytes;
impl SizeUnit for Bytes {
    fn singular_name() -> String {
        "B".to_string()
    }
    fn num_byte_in_unit() -> u64 {
        1
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Kilobytes;
impl SizeUnit for Kilobytes {
    fn singular_name() -> String {
        "KB".to_string()
    }
    fn num_byte_in_unit() -> u64 {
        1_024
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Filesize<T: SizeUnit> {
    bytes: u64,
    unit: PhantomData<T>,
}

impl<T> From<u64> for Filesize<T>
where
    T: SizeUnit,
{
    fn from(n: u64) -> Self {
        Filesize {
            bytes: n * T::num_byte_in_unit(),
            unit: PhantomData,
        }
    }
}

impl<T> From<Filesize<T>> for u64
where
    T: SizeUnit,
{
    fn from(fsz: Filesize<T>) -> u64 {
        ((fsz.bytes as f64) / (T::num_byte_in_unit() as f64)) as u64
    }
}

impl<T> fmt::Display for Filesize<T>
where
    T: SizeUnit,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // convert value in associated units to float
        let size_val = ((self.bytes as f64) / (T::num_byte_in_unit() as f64)) as u64;

        // plural?
        let name_plural = match size_val {
            1 => "",
            _ => "s",
        };

        write!(
            f,
            "{} {}{}",
            (self.bytes as f64) / (T::num_byte_in_unit() as f64),
            T::singular_name(),
            name_plural
        )
    }
}

// Can be expanded for From<File>, or any type that has an alias for Metadata
impl<T> From<&PathBuf> for Filesize<T>
where
    T: SizeUnit,
{
    fn from(f: &PathBuf) -> Self {
        Filesize {
            bytes: f
                .metadata()
                .expect("error with metadata from pathbuf into filesize")
                .len(),
            unit: PhantomData,
        }
    }
}

impl<T> From<fs::File> for Filesize<T>
where
    T: SizeUnit,
{
    fn from(f: fs::File) -> Self {
        Filesize {
            bytes: f
                .metadata()
                .expect("error with metadata from file into filesize")
                .len(),
            unit: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_bytes() {
        use std::fs;
        use std::io::prelude::*;

        let mut file = fs::File::create("testing_11.txt").expect("could not create test file");
        file.write_all(b"hello world")
            .expect("could not write to test file"); // 11 bytes

        let size = Filesize::<Bytes>::from(file);
        assert_eq!(size.bytes, 11_u64);

        fs::remove_file("testing_11.txt").expect("could not remove test file");
    }

    #[test]
    fn test_output_thousand() {
        use std::fs;
        use std::io::prelude::*;

        let mut file = fs::File::create("testing_1000.txt").expect("could not create test file");
        file.write_all(b"Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Donec quam felis, ultricies nec, pellentesque eu, pretium quis, sem. Nulla consequat massa quis enim. Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu. In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo. Nullam dictum felis eu pede mollis pretium. Integer tincidunt. Cras dapibus. Vivamus elementum semper nisi. Aenean vulputate eleifend tellus. Aenean leo ligula, porttitor eu, consequat vitae, eleifend ac, enim. Aliquam lorem ante, dapibus in, viverra quis, feugiat a, tellus. Phasellus viverra nulla ut metus varius laoreet. Quisque rutrum. Aenean imperdiet. Etiam ultricies nisi vel augue. Curabitur ullamcorper ultricies nisi. Nam eget dui. Etiam rhoncus. Maecenas tempus, tellus eget condimentum rhoncus, sem quam semper libero, sit amet adipiscing sem neque sed ipsum. N")
            .expect("could not write to test file"); // 1000 bytes (1 KB)

        let size = Filesize::<Bytes>::from(file);
        assert_eq!(size.bytes, 1000_u64);

        fs::remove_file("testing_1000.txt").expect("could not remove test file");
    }
}
