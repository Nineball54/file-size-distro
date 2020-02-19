use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, io, time};
use walkdir::{DirEntry, WalkDir};

fn main() -> Result<(), Box<dyn Error>> {
    let start = time::Instant::now();
    let args: Vec<String> = env::args().collect();

    // Assign root from cmd input
    let root: &Path = parse_path(&args).expect("not a valid path");
    // Recursively build directory
    let dir = WalkDir::new(&root);
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = {
        // Retrieve all entries from WalkDir
        let pool = pool(dir).expect("unable to retrieve entries from WalkDir");
        // check and pull all paths that are files, seperating from all paths that are directories
        partition_from(pool).expect("unable to partition files from directories")
    };

    let header = format!("++ File size distribution for : {} ++\n", &root.display());

    let (fcount, dcount): (usize, usize) = (files.len(), dirs.len());
    let (size_by_count, total_size): ([u64; 6], u64) = file_count(files);
    let out_size = format!(
        "\nFiles @  0B            : {}\nFiles >  1B - 1,023B   : {}\nFiles > 1KB - 1,023KB  : {}\nFiles > 1MB - 1,023MB  : {}\nFiles > 1GB - 1,023GB  : {}\nFiles > 1TB+           : {}\n",
        size_by_count[0], size_by_count[1], size_by_count[2], size_by_count[3], size_by_count[4], size_by_count[5],
    );

    let out_stat = format!(
        "\nTotal number of files counted: {}\nTotal number of directories traversed: {}\nTotal size of all files: {}\n",
        fcount, dcount, total_size
    );

    let end = time::Instant::now();
    let run_len = format!("\nRun time: {:?}\n", end.duration_since(start));

    println!("{}{}{}{}", header, out_size, out_stat, run_len);
    Ok(())
}

// If there's no `args` entered, the executable will search it's own path.
fn parse_path(args: &[String]) -> Result<&Path, io::Error> {
    if args.len() == 1 {
        Ok(Path::new(&args[0]))
    } else {
        Ok(Path::new(&args[1]))
    }
}

fn pool(dir: WalkDir) -> Result<Vec<DirEntry>, Box<dyn Error>> {
    // Take the iter `WalkDir` and check each item for errors, dropping all invalid `DirEntry`s
    Ok(dir.into_iter().filter_map(|e| e.ok()).collect())
}

fn partition_from(pool: Vec<DirEntry>) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
    // With each `DirEntry`, pull the `Path` from it, then check what kind of `File` the `Path` points at.
    Ok(pool
        .into_iter()
        .map(|e| e.into_path())
        .partition(|path| path.is_file()))
}

fn file_count(files: Vec<PathBuf>) -> ([u64; 6], u64) {
    let mut fc_by_size: [u64; 6] = [0; 6];
    for file in &files {
        // metadata().len() returns u64 / bytes
        match file
            .metadata()
            .expect("error with metadata while matching")
            .len()
        {
            // Empty
            0 => fc_by_size[0] += 1,
            // 1 byte to 999 bytes
            1u64..=1023u64 => fc_by_size[1] += 1,
            // 1kb to 0.99 kb
            1024u64..=1_048_575_u64 => fc_by_size[2] += 1,
            // 1 mb to 0.99 mb
            1_048_576_u64..=1_073_741_823_u64 => fc_by_size[3] += 1,
            // 1 gb to 0.99 gb
            1_073_741_824_u64..=109_951_162_775_u64 => fc_by_size[4] += 1,
            // 1 tb or larger
            109_951_162_776_u64..=std::u64::MAX => fc_by_size[5] += 1,
        };
    }

    let total_file_size: u64 = files.iter().fold(0, |acc, f| {
        acc + f
            .metadata()
            .expect("error with metadata while folding")
            .len()
    });
    (fc_by_size, total_file_size)
}
