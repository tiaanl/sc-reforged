#[derive(PartialEq, Eq, Hash)]
pub enum ModelName {
    Object(String),
    Body(String),
    Head(String),
    _Misc(String),
    BodyDefinition(String, String), // (<profile name>, <body def name>)
}
