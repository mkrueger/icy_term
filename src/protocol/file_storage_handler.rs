use std::collections::HashMap;

pub trait FileStorageHandler
where
    Self: std::marker::Send,
{
    fn open_file(&mut self, file_name: &str, total_size: usize);
    fn append(&mut self, data: &[u8]);
    fn close(&mut self);

    fn current_file_name(&self) -> Option<String>;
    fn current_file_length(&self) -> usize;
    fn set_current_size_to(&mut self, size: usize);
    fn get_current_file_total_size(&self) -> usize;
}

#[derive(Clone)]
pub struct TestStorageHandler {
    cur_file_name: Option<String>,
    cur_file_size: usize,
    pub file: HashMap<String, Vec<u8>>,
}

impl TestStorageHandler {
    pub fn new() -> Self {
        Self {
            cur_file_name: None,
            cur_file_size: 0,
            file: HashMap::new(),
        }
    }
}

impl FileStorageHandler for TestStorageHandler {
    fn open_file(&mut self, file_name: &str, total_size: usize) {
        let fn_string = file_name.to_string();
        self.cur_file_name = Some(fn_string.clone());
        self.cur_file_size = total_size;
        self.file.insert(fn_string, Vec::new());

        println!("open file: {file_name} with size {total_size}");
    }

    fn current_file_name(&self) -> Option<String> {
        self.cur_file_name.clone()
    }

    fn set_current_size_to(&mut self, size: usize) {
        if let Some(file_name) = &self.cur_file_name {
            println!("cut from {} to {} bytes.", self.current_file_length(), size);
            self.file.get_mut(file_name).unwrap().resize(size, 0);
        }
    }

    fn append(&mut self, data: &[u8]) {
        println!("append {} bytes.", data.len());
        if let Some(file_name) = &self.cur_file_name {
            self.file
                .get_mut(file_name)
                .unwrap()
                .extend_from_slice(data);
        }
    }
    fn close(&mut self) {
        println!("close file.");
        self.cur_file_name = None;
        self.cur_file_size = 0;
    }

    fn current_file_length(&self) -> usize {
        if let Some(file_name) = &self.cur_file_name {
            self.file.get(file_name).unwrap().len()
        } else {
            0
        }
    }
    fn get_current_file_total_size(&self) -> usize {
        self.cur_file_size
    }
}
