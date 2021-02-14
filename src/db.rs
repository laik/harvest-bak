use super::Result;
use crate::event_handler::EventHandler;
use std::collections::HashMap;

macro_rules! enum_str {
    (enum $name:ident {
        $($variant:ident = $val:expr),*,
    }) => {
        enum $name {
            $($variant = $val),*
        }

        impl $name {
            fn name(&self) -> &'static str {
                match self {
                    $($name::$variant => stringify!($variant)),*
                }
            }
        }
    };
}

enum_str! {
    enum Event {
        Add = 0x00,
        Delete = 0x01,
        Update = 0x02,
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    // unique identifier path
    uuid: String,
    offset: usize,
    inode: usize,
    namespace: String,
    pod: String,
    container: String,
}

pub struct Database {
    // rows key is the rows uuid
    rows: HashMap<String, Row>,
    // row op wal log
    event_handler: EventHandler<Row>,
}

impl Database {
    pub fn new(event_handler: EventHandler<Row>) -> Self {
        Self {
            rows: HashMap::new(),
            event_handler,
        }
    }

    pub fn all(&mut self) -> Result<Vec<Row>> {
        let mut result = vec![];
        for (_, v) in self.rows.iter() {
            result.push(v.clone())
        }
        Ok(result)
    }

    pub fn get_by_uuid(&self, uuid: String) -> Option<&Row> {
        self.rows.get(&*uuid)
    }

    pub fn add_row(&mut self, uuid: String, row: Row) -> Result<()> {
        self.event_handler
            .publish(Event::Add.name().parse().unwrap(), row.clone());
        self.rows.insert(uuid, row);
        Ok(())
    }

    pub fn delete_row(&mut self, uuid: String) -> Result<()> {
        match self.rows.get(&*uuid) {
            Some(row) => {
                self.event_handler
                    .publish(Event::Delete.name().parse().unwrap(), row.clone());
                self.rows.remove(&*uuid);
            }
            None => {}
        }
        Ok(())
    }

    pub fn update_row(&mut self, uuid: String, row: Row) -> Result<()> {
        self.event_handler
            .publish(Event::Update.name().parse().unwrap(), row.clone());
        self.rows.insert(uuid, row);
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {}
}
