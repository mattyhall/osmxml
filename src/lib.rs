#![feature(globs)]
#![feature(struct_variant)]
extern crate sax;

use std::collections::HashMap;

type Tags<'a> = HashMap<&'a str, &'a str>;

#[deriving(Show)]
pub enum OsmElement<'a> {
    Node{id: int, lat: f64, lng: f64, visible: bool, tags: Tags<'a>},
    Way{id: int, nodes: Vec<int>},
}

pub struct Osm<'a> {
    pub parser: Receiver<sax::ParseResult>,
    pub elements: HashMap<int, OsmElement<'a>>,
}

impl<'a> Osm<'a> {
    pub fn new(path: &Path) -> Osm {
        let parser = sax::parse_file(path).unwrap();
        Osm {parser: parser, elements: HashMap::new()}
    }
    
    pub fn parse(&mut self) {
        match self.parser.iter().next().unwrap() {
            Ok(sax::StartDocument) => (),
            _                      => fail!("Parsing failed")
        }

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => self.parse_start_element(name, attrs),
                Ok(_) => {},
                Err(e) => fail!("{}", e),
            }
        }
    }

    fn parse_start_element(&mut self, name: String, attrs: sax::Attributes) {
        match name.as_slice() {
            "relation" => self.parse_relation(),
            "node" => self.parse_node(attrs),
            "way" => self.parse_way(attrs),
            _  => println!("Skipping {} tag", name)
        }
    }

    fn parse_node(&mut self, attrs: sax::Attributes) {
        let id = from_str(attrs.get("id")).unwrap();
        let lat = from_str(attrs.get("lat")).unwrap();
        let lng = from_str(attrs.get("lon")).unwrap();
        let visible = from_str(attrs.get("visible")).unwrap();
        self.elements.insert(id, Node{id: id, lat: lat, lng: lng, visible: visible, tags: HashMap::new()});

        let mut depth: int = 1;
        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(..)) => depth += 1,
                Ok(sax::EndElement(..)) => {
                    depth -= 1;
                    if depth <= 0 {
                        return;
                    }
                }
                _ => (),
            }
        }
    }

    fn parse_way(&mut self, attrs: sax::Attributes) {
        let id = from_str(attrs.get("id")).unwrap();
        let mut nodes = Vec::new();
        let mut depth: int = 1;
        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    depth += 1;
                    match name.as_slice() {
                        "nd" => nodes.push(self.parse_nd(attrs)),
                        _ => (),
                    }
                }
                Ok(sax::EndElement(..)) => {
                    depth -= 1;
                    if depth <= 0 {
                        break;
                    }
                }
                _ => (),
            }
        }
        self.elements.insert(id, Way {id: id, nodes: nodes});
    }

    fn parse_nd(&mut self, attrs: sax::Attributes) -> int {
        from_str(attrs.get("ref")).unwrap()
    }

    fn parse_relation(&mut self) {
        println!("Got relation");
    }
}
