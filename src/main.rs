extern crate flate2;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::env::args;
use std::fs;
use std::io;
use std::time::Instant;
use zip;

fn main() {
    let exit_code = real_main();
    std::process::exit(exit_code);
}

fn real_main() -> i32 {
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
    println!("{}", mode);
    if mode == "compress" {
        println!("Compressing...");
        compress();
    } else if mode == "decompress" {
        println!("Deompressing...");
        decompress(args.clone());
    } else {
        eprintln!("Invalid mode");
        return 2;
    }

    return 0;
}

fn compress() {
    // open and read the file
    let mut input = io::BufReader::new(fs::File::open(args().nth(1).unwrap()).unwrap());

    // set the file output name
    let output = fs::File::create(args().nth(2).unwrap()).unwrap();

    let mut encoder = GzEncoder::new(output, Compression::best());
    let start = Instant::now();
    io::copy(&mut input, &mut encoder).unwrap();
    let output = encoder.finish().unwrap();
    println!(
        "Source len: {:?}",
        input.get_ref().metadata().unwrap().len()
    );

    println!("Target len: {:?}", output.metadata().unwrap().len());

    println!("Elapsed time: {:?}", start.elapsed())
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
