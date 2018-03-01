extern crate flate2;
extern crate xml;

use std::fs::File;
use std::io::BufReader;
use flate2::bufread::GzDecoder;

use xml::reader::{EventReader, XmlEvent};

fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

fn main() {
    // available from https://dumps.wikimedia.org/enwiki/enwiki-latest-abstract.xml.gz
    let file = File::open("enwiki-latest-abstract.xml.gz").unwrap();
    let file = BufReader::new(file);
    let file = GzDecoder::new(file);

    let parser = EventReader::new(file);
    let mut depth = 0;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                println!("{}+{}", indent(depth), name);
                depth += 1;
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                println!("{}-{}", indent(depth), name);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
