#![feature(globs, macro_rules, struct_variant, slicing_syntax)]
extern crate xml;

use std::collections::HashMap;
use std::io::{IoError, File, BufferedReader};
use xml::reader::EventReader;
use xml::common::Attribute;
use xml::reader::events::*;

pub type Tags = HashMap<String, String>;

#[deriving(Show)]
pub enum OsmParseError {
    IoErr(IoError),
    ParseErr(String),
}

pub type ParseResult = Result<(), OsmParseError>;

#[deriving(Show)]
pub enum OsmElement {
    Node {
        pub id: int,
        pub lat: f64, pub lng: f64,
        pub visible: bool, pub tags: Tags
    },
    Way { pub id: int, pub nodes: Vec<int>, pub tags: Tags },
    Relation { pub id: int, pub members: Vec<int>, pub tags: Tags },
}

macro_rules! parse {
    ($parser:expr, $close_tag:expr $(, $tag:pat => $method:expr)*) => {
        loop {
            match $parser.next() {
                StartElement {name, attributes, ..} => {
                    match name.local_name[] {
                        $($tag => try!($method(attributes)),)*
                        _ => return Err(ParseErr(format!(
                              "Unexpected child in {} Got a {}",
                              $close_tag, name.local_name))),
                    }
                }
                EndElement {name, ..} => {
                    if name.local_name[] == $close_tag {
                        break;
                    }
                    return Err(ParseErr(format!(
                            "Expecting {} to end, not a {}", 
                            $close_tag, name.local_name)));
                }
                EndDocument => return Err(ParseErr("Premature end".to_string())),
                _ => {},
            }
        }
    }
}

fn find_attribute<'a>(attributes: &'a Vec<Attribute>, s: &str) -> Option<&'a str> {
    attributes.iter().find(|attr| attr.name.local_name[] == s)
                     .map(|v| v.value[])
}

pub struct Osm {
    parser: EventReader<BufferedReader<File>>,
    pub elements: HashMap<int, OsmElement>,
}

impl Osm {
    pub fn new(path: &Path) -> Result<Osm, OsmParseError> {
        let file = File::open(path).unwrap();
        let reader = BufferedReader::new(file);
        let parser = EventReader::new(reader);
        let mut s = Osm {parser: parser, elements: HashMap::new()};
        try!(s.parse());
        Ok(s)
    }
    
    fn parse(&mut self) -> ParseResult {
        loop {
            match self.parser.next() {
                StartElement {name, attributes, ..} => try!(self.parse_start_element(name.local_name, attributes)),
                // Should only skip comments/text as each parse function is responsible for
                // matching it's ending event
                EndDocument => break,
                _ => (),
            }
        }
        Ok(())
    }

    fn parse_start_element(&mut self, name: String, attributes: Vec<Attribute>) -> ParseResult {
        match name.as_slice() {
            "relation" => try!(self.parse_relation(attributes)),
            "node" => try!(self.parse_node(attributes)),
            "way" => try!(self.parse_way(attributes)),
            _  => ()
        }
        Ok(())
    }

    fn parse_node(&mut self, attributes: Vec<Attribute>) -> ParseResult { 
        let id  = find_attribute(&attributes, "id").and_then(|v| from_str(v));
        let lat  = find_attribute(&attributes, "lat").and_then(|v| from_str(v));
        let lng  = find_attribute(&attributes, "lon").and_then(|v| from_str(v));
        let visible  = find_attribute(&attributes, "visible").and_then(|v| from_str(v));
        let (id, lat, lng, visible) = match (id, lat, lng, visible) {
            (Some(id), Some(lat), Some(lng), Some(visible)) => (id, lat, lng, visible),
            _ => return Err(ParseErr(
                "Could not find all required attributes on node".to_string()))
        };
        let mut tags = HashMap::new();

        parse!(self.parser, "node", 
               "tag" => |attributes| self.parse_tag(attributes, &mut tags));

        self.elements.insert(id, Node{id: id, lat: lat, lng: lng,
                                      visible: visible, tags: tags});

        Ok(())
    }

    fn parse_int_attr(&self, k: &str, attributes: Vec<Attribute>) -> Result<int, OsmParseError> {
        return match find_attribute(&attributes, k).and_then(|v| from_str(v)) {
            Some(id) => Ok(id),
            None => Err(ParseErr("Could not find id/ref attribute".to_string())),
        };
    }

    fn parse_way(&mut self, attributes: Vec<Attribute>) -> ParseResult {
        let id = try!(self.parse_int_attr("id", attributes));
        let mut nodes = Vec::new();
        let mut tags = HashMap::new();

        parse!(self.parser, "way",
               "nd" => |attributes| Ok(nodes.push(try!(self.parse_nd(attributes)))),
               "tag" => |attributes| self.parse_tag(attributes, &mut tags));

        self.elements.insert(id, Way {id: id, nodes: nodes, tags: tags});
        Ok(())
    }

    fn parse_nd(&mut self, attributes: Vec<Attribute>) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attributes));
        parse!(self.parser, "nd");
        Ok(i)
    }

    fn parse_tag(&mut self, attributes: Vec<Attribute>, tags: &mut Tags) -> ParseResult {
        let (k, v) = match (find_attribute(&attributes, "k"), find_attribute(&attributes, "v")) {
            (Some(k), Some(v)) => (k, v),
            _ => return Err(ParseErr("Tag must have a k and a v attribute".to_string()))
        };

        parse!(self.parser, "tag");

        tags.insert(k.to_string(), v.to_string());
        Ok(())
    }

    fn parse_member(&mut self, attributes: Vec<Attribute>) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attributes));

        parse!(self.parser, "member");

        Ok(i)
    }

    fn parse_relation(&mut self, attributes: Vec<Attribute>) -> ParseResult {
        let id = try!(self.parse_int_attr("id", attributes));
        let mut tags = HashMap::new();
        let mut members = Vec::new();

        parse!(self.parser, "relation",
               "member" => |attributes| Ok(members.push(try!(self.parse_member(attributes)))),
               "tag" => |attributes| self.parse_tag(attributes, &mut tags));

        self.elements.insert(id, Relation{id: id, members: members, tags: tags});
        Ok(())
    }
}
