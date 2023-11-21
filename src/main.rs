use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::{Seek, Write};
use std::iter::Iterator;
use std::time::Instant;
use walkdir::DirEntry;
use zip;

fn main() {
    let exit_code = real_main();
    std::process::exit(exit_code);
}

fn real_main() -> i32 {
    // TODO: CLI usage should be:
    // compress_util -o (compress/decompress) -s (source file/directory) [-d] (destination, should be optional, by degfault to extract in the current working directory)

    let args: Vec<_> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: `source` `target` [`mode`]");
        return 1;
    }

    // if mode is not passed, set default mode to compress
    let mode: String;
    if args.len() == 3 {
        mode = String::from("compress");
    } else {
        mode = args[3].to_string();
    }

    let start = Instant::now();

    if mode == "compress" {
        println!("Compressing...");
        compress(args.clone());
    } else if mode == "decompress" {
        println!("Deompressing...");
        decompress(args.clone());
    } else {
        eprintln!("Invalid mode");
        return 2;
    }
    println!("Elapsed time: {:?}", start.elapsed());
    return 0;
}

fn compress(args: Vec<String>) {
    let src_dir = &*args[1];
    let dst_file = &*args[2];
    match start_compression(src_dir, dst_file, zip::CompressionMethod::Zstd) {
        Ok(_) => println!("done: {src_dir} written to {dst_file}"),
        Err(e) => println!("Error: {e:?}"),
    };
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    let options = zip::write::FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        // let name = path;
        let name = path.strip_prefix(std::path::Path::new(prefix)).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            println!("adding file {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = fs::File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            println!("adding dir {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()?;
    Result::Ok(())
}

fn start_compression(
    src_dir: &str,
    dst_file: &str,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()> {
    let src = std::path::Path::new(src_dir);

    if src.is_dir() {
        // return Err(zip::result::ZipError::FileNotFound);
        let path = std::path::Path::new(dst_file);
        let file = fs::File::create(path).unwrap();

        let walkdir = walkdir::WalkDir::new(src_dir);
        let it = walkdir.into_iter();

        zip_dir(&mut it.filter_map(|e| e.ok()), "", file, method)?;

        return Ok(());
    } else if src.is_file() {
        // TODO: compress file
        return Ok(());
    }

    return Err(zip::result::ZipError::FileNotFound);
}

fn decompress(args: Vec<String>) {
    let fname = std::path::Path::new(&args[1]);
    let file = fs::File::open(&fname).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (*file.name()).ends_with('/') {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );

            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }

            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
}
