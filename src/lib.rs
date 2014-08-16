#![feature(globs)]
#![feature(struct_variant)]
extern crate sax;

use std::collections::HashMap;

type Tags = HashMap<String, String>;

#[deriving(Show)]
pub enum OsmElement {
    Node{id: int, lat: f64, lng: f64, visible: bool, tags: Tags},
    Way{id: int, nodes: Vec<int>, tags: Tags},
}

pub struct Osm {
    pub parser: Receiver<sax::ParseResult>,
    pub elements: HashMap<int, OsmElement>,
}

impl Osm {
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
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "tag" => self.parse_tag(attrs, &mut tags),
                        _ => fail!("Wrong thing"),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "node" {
                        break;
                    }
                    fail!("Wrong thing");
                }
                _ => (),
            }
        }

        self.elements.insert(id, Node{id: id, lat: lat, lng: lng, visible: visible, tags: tags});
    }

    fn parse_way(&mut self, attrs: sax::Attributes) {
        let id = from_str(attrs.get("id")).unwrap();
        let mut nodes = Vec::new();
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "nd" => nodes.push(self.parse_nd(attrs)),
                        "tag" => self.parse_tag(attrs, &mut tags),
                        _ => fail!("Wrong thing {} {}", name, attrs),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "way" {
                        break;
                    }
                    fail!("Wrong thing");
                }
                _ => (),
            }
        }
        self.elements.insert(id, Way {id: id, nodes: nodes, tags: tags});
    }

    fn parse_nd(&mut self, attrs: sax::Attributes) -> int {
        let i = from_str(attrs.get("ref")).unwrap();

        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "nd" {
                        break;
                    }
                    fail!("Wrong thing");
                }
                _ => fail!("Wrong thing")
            }
        }
        return i;
    }

    fn parse_tag(&mut self, attrs: sax::Attributes, tags: &mut Tags) {
        let (k, v) = (attrs.get_clone("k"), attrs.get_clone("v"));
        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "tag" {
                        tags.insert(k, v);
                        return;
                    }
                }
                _ => fail!("Got wrong thing"),
            }
        }
    }

    fn parse_relation(&mut self) {
    }
}
