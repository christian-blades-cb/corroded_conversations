// extern crate flate2;
extern crate bzip2;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate xml;

use std::fs::File;
use std::io::BufReader;
// use flate2::bufread::GzDecoder;
use bzip2::bufread::BzDecoder;
use std::env::args;
use slog::*;
use xml::name::OwnedName;
use nom::{alphanumeric, whitespace};

use xml::reader::{EventReader, XmlEvent};

lazy_static! {
    static ref LOG: slog::Logger = {
        let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
        let logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!("app" => "corroded_conversations"));
        logger
    };
}

enum WikiState {
    WaitingForPage,
    InPage,
    InTitle,
    InText,
}

#[derive(Debug)]
struct Article {
    title: String,
    text: String,
}

fn do_wait_for_page(
    state: &mut WikiState,
    event: std::result::Result<XmlEvent, xml::reader::Error>,
    article: &mut Article,
) {
    match event {
        Ok(XmlEvent::StartElement {
            name:
                OwnedName {
                    local_name: elem_name,
                    ..
                },
            ..
        }) => {
            if elem_name == "page" {
                trace!(LOG, "start of page element");
                *state = WikiState::InPage;
                *article = Article {
                    title: String::new(),
                    text: String::new(),
                };
            }
        }
        // XXX: handle XmlEvent::EndDocument ??
        _ => {}
    }
}

fn do_in_page(
    state: &mut WikiState,
    event: std::result::Result<XmlEvent, xml::reader::Error>,
    article: &mut Article,
) {
    match event {
        Ok(XmlEvent::EndElement {
            name:
                OwnedName {
                    local_name: elem_name,
                    ..
                },
            ..
        }) => {
            if elem_name == "page" {
                debug!(LOG, "article"; "title" => &article.title, "text" => &article.text);
                *state = WikiState::WaitingForPage;
            }
        }
        Ok(XmlEvent::StartElement {
            name:
                xml::name::OwnedName {
                    local_name: elem_name,
                    ..
                },
            ..
        }) => match elem_name.as_ref() {
            "title" => *state = WikiState::InTitle,
            "text" => *state = WikiState::InText,
            _ => {}
        },
        _ => {}
    }
}

fn do_in_title(
    state: &mut WikiState,
    event: std::result::Result<XmlEvent, xml::reader::Error>,
    article: &mut Article,
) {
    match event {
        Ok(XmlEvent::EndElement {
            name:
                OwnedName {
                    local_name: elem_name,
                    ..
                },
            ..
        }) => {
            if elem_name == "title" {
                *state = WikiState::InPage;
            }
        }
        Ok(XmlEvent::Characters(text)) => article.title.push_str(&text),
        _ => {}
    }
}

fn do_in_text(
    state: &mut WikiState,
    event: std::result::Result<XmlEvent, xml::reader::Error>,
    article: &mut Article,
) {
    match event {
        Ok(XmlEvent::EndElement {
            name:
                OwnedName {
                    local_name: elem_name,
                    ..
                },
            ..
        }) => {
            if elem_name == "text" {
                *state = WikiState::InPage;
            }
        }
        Ok(XmlEvent::Characters(text)) => {
            // trace!(LOG, "found characters for article text"; "text" => &text);
            article.text.push_str(&text)
        }
        _ => {}
    }
}

fn main() {
    let filename = args()
        .nth(1)
        .unwrap_or("enwiki-latest-pages-articles.xml.bz2".to_owned());
    // available from https://dumps.wikimedia.org/enwiki/enwiki-latest-abstract.xml.gz
    info!(LOG, "processing wikipedia articles"; "filename" => &filename);
    let file = File::open(filename).expect("unable to open file");
    let file = BufReader::new(file);
    let file = BzDecoder::new(file);

    let mut state = WikiState::WaitingForPage;
    let parser = EventReader::new(file);
    let mut current_article = Article {
        title: String::new(),
        text: String::new(),
    };
    for e in parser {
        match state {
            WikiState::WaitingForPage => do_wait_for_page(&mut state, e, &mut current_article),
            WikiState::InPage => do_in_page(&mut state, e, &mut current_article),
            WikiState::InTitle => do_in_title(&mut state, e, &mut current_article),
            WikiState::InText => do_in_text(&mut state, e, &mut current_article),
        }
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    link,
    delimited!(
        tag!("[["),
        do_parse!(
            link: words >>
            opt!(description) >>
            link
        ),
        tag!("]]")
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(description<&[u8], Vec<&[u8]>>,
       do_parse!(
           tag!("|") >>
           descr: many1!(
                   alt!(words | link)
           ) >>
           (descr)
       )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(words<&[u8], &[u8]>,
       take_while!(not_a_delim)
);

fn not_a_delim(chr: u8) -> bool {
    if chr == '|' as u8 || chr == ']' as u8 {
        false
    } else {
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_links() {
        let input = b"[[File:Tizi Ouzou Tasdawit.jpg|thumb|Signs in the [[University of Tizi Ouzou]] in three languages: [[Arabic]], [[Berber languages|Berber]], and French]]";
        let (_, val) = link(input).unwrap();

        let expected = b"File:Tizi Ouzou Tasdawit.jpg|thumb|Signs in the [[University of Tizi Ouzou]] in three languages: [[Arabic]], [[Berber languages|Berber]], and French";
        assert_eq!(val[..], expected[..]);
    }
}
