use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

pub trait ValueHandler {
    fn parse_value(&self, value: &str) -> Result<(), Error>;
    fn requires_value(&self) -> bool;
    fn set_value(&self);
}

pub struct IntParameter {
    value: Cell<isize>,
}

impl IntParameter {
    pub fn new(value: isize) -> IntParameter {
        IntParameter { value: Cell::new(value) }
    }

    pub fn get_value(&self) -> isize {
        self.value.get()
    }
}

impl ValueHandler for IntParameter {
    fn parse_value(&self, value: &str) -> Result<(), Error> {
        self.value.set(isize::from_str(value).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?);
        Ok(())
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }
}

pub struct StringParameter {
    value: RefCell<String>,
}

impl StringParameter {
    pub fn new(value: &str) -> StringParameter {
        StringParameter { value: RefCell::new(value.to_string()) }
    }

    pub fn get_value(&self) -> String {
        self.value.borrow().clone()
    }
}

impl ValueHandler for StringParameter {
    fn parse_value(&self, value: &str) -> Result<(), Error> {
        *self.value.borrow_mut() = value.to_string();
        Ok(())
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }
}

pub struct BoolParameter {
    value: Cell<bool>,
}

impl BoolParameter {
    pub fn new() -> BoolParameter {
        BoolParameter { value: Cell::new(false) }
    }

    pub fn get_value(&self) -> bool {
        self.value.get()
    }
}

impl ValueHandler for BoolParameter {
    fn parse_value(&self, _value: &str) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "should not be called"))
    }

    fn requires_value(&self) -> bool {
        return false;
    }

    fn set_value(&self) {
        self.value.set(true);
    }
}

pub struct SizeParameter {
    value: Cell<isize>,
}

impl SizeParameter {
    pub fn new(value: isize) -> SizeParameter {
        SizeParameter { value: Cell::new(value) }
    }

    pub fn get_value(&self) -> isize {
        self.value.get()
    }
}

impl ValueHandler for SizeParameter {
    fn parse_value(&self, value: &str) -> Result<(), Error> {
        let multiplier = match value.chars().last().ok_or(Error::new(ErrorKind::InvalidInput, "invalid size parameter"))? {
            'M' => 1024 * 1024,
            'K' => 1024,
            'G' => 1024 * 1024 * 1024,
            _ => 1
        };
        let size = if multiplier == 1 {
            isize::from_str(value).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?
        } else {
            let mut chars = value.chars();
            chars.next_back();
            isize::from_str(chars.as_str()).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?
        };
        self.value.set(size * multiplier);
        Ok(())
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }
}

#[derive(Clone)]
pub struct Switch<'a> {
    name: String,
    switch: Option<char>,
    ext_switch: Option<String>,
    handler: &'a dyn ValueHandler,
}

impl<'a> Switch<'a> {
    pub fn new(name: &str, switch: Option<char>, ext_switch: Option<&str>, handler: &'a dyn ValueHandler) -> Switch<'a> {
        Switch {
            name: name.to_string(),
            switch,
            ext_switch: ext_switch.map(|s|s.to_string()),
            handler,
        }
    }

    fn to_string(&self) -> String {
        let mut result = "".to_string();
        if self.handler.requires_value() {
            if let Some(sw) = self.switch {
                result.push_str(format!(" -{} {}", sw, self.name).as_str());
            }
            if let Some(sw) = &self.ext_switch {
                result.push_str(format!(" --{} {}", sw, self.name).as_str());
            }
        } else {
            if let Some(sw) = self.switch {
                result.push_str(format!(" -{}", sw).as_str());
            }
            if let Some(sw) = &self.ext_switch {
                result.push_str(format!(" --{}", sw).as_str());
            }
        }
        if result.len() == 0 {
            return self.name.clone();
        }
        result
    }

    fn parse_value(&self, value: &str) -> Result<(), Error> {
        self.handler.parse_value(value)?;
        Ok(())
    }

    fn requires_value(&self) -> bool {
        self.handler.requires_value()
    }

    fn set_value(&self) {
        self.handler.set_value()
    }
}

pub struct Arguments<'a> {
    program_name: String,
    switch_map: HashMap<char, Switch<'a>>,
    ext_switch_map: HashMap<String, Switch<'a>>,
    other_arguments: Vec<String>,
}

impl<'a> Arguments<'a> {
    pub fn new(program_name: &str, switches: &[Switch<'a>]) -> Arguments<'a> {
        let mut switch_map = HashMap::new();
        let mut ext_switch_map = HashMap::new();
        for switch in switches {
            if let Some(sw) = switch.switch {
                switch_map.insert(sw, switch.clone());
            }
            if let Some(sw) = &switch.ext_switch {
                ext_switch_map.insert(sw.clone(), switch.clone());
            }
        }
        Arguments {
            program_name: program_name.to_string(),
            switch_map,
            ext_switch_map,
            other_arguments: Vec::new(),
        }
    }

    pub fn usage(&self) {
        let mut usage = "Usage: ".to_string();
        usage.push_str(&self.program_name);
        for (_name, sw) in &self.switch_map {
            usage.push_str(sw.to_string().as_str())
        }
        for (_name, sw) in &self.ext_switch_map {
            usage.push_str(sw.to_string().as_str())
        }
        println!("{}", usage);
    }

    pub fn build(&mut self, args: Vec<String>) -> Result<(), Error> {
        let mut current_parameter: Option<&Switch> = None;
        for arg in args {
            if let Some(p) = current_parameter {
                p.parse_value(arg.as_str())?;
                current_parameter = None;
            } else {
                if arg.starts_with('-') {
                    if arg.starts_with("--") {
                        if arg.len() == 2 {
                            return Err(Error::new(ErrorKind::InvalidInput, "invalid ext_switch"));
                        }
                        if let Some(p) = self.ext_switch_map.get(&arg.chars().skip(2).collect::<String>()) {
                            if p.requires_value() {
                                current_parameter = Some(p);
                            } else {
                                p.set_value();
                            }
                        } else {
                            return Err(Error::new(ErrorKind::InvalidInput, "unknown ext switch"));
                        }
                    } else {
                        if arg.len() != 2 {
                            return Err(Error::new(ErrorKind::InvalidInput, "invalid switch"));
                        }
                        if let Some(p) = self.switch_map.get(&arg.chars().skip(1).next().unwrap()) {
                            if p.requires_value() {
                                current_parameter = Some(p);
                            } else {
                                p.set_value();
                            }
                        } else {
                            return Err(Error::new(ErrorKind::InvalidInput, "unknown switch"));
                        }
                    }
                } else {
                    self.other_arguments.push(arg.clone());
                }
            }
        }
        if current_parameter.is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, "switch value expected"));
        }
        Ok(())
    }

    pub fn get_other_arguments(&self) -> &Vec<String> {
        &self.other_arguments
    }
}

#[cfg(test)]
mod tests {
    use crate::{Arguments, BoolParameter, IntParameter, SizeParameter, StringParameter, Switch};

    #[test]
    fn test_arguments_parser() {
        let port_parameter = IntParameter::new(6379);
        let max_memory_parameter = SizeParameter::new(1024 * 1024 * 1024);//1G
        let threads_parameter = IntParameter::new(4);
        let verbose_parameter = BoolParameter::new();
        let string_parameter = StringParameter::new("init");
        let switches = [
            Switch::new("port", Some('p'), None, &port_parameter),
            Switch::new("maximum_memory", Some('m'), None, &max_memory_parameter),
            Switch::new("threads", Some('t'), None, &threads_parameter),
            Switch::new("verbose", Some('v'), None, &verbose_parameter),
            Switch::new("test", None, Some("ss"), &string_parameter),
        ];
        let mut arguments = Arguments::new("cache", &switches);
        assert!(arguments.build(vec![
            "-p".to_string(), "3333".to_string(),
            "-m".to_string(), "1M".to_string(),
            "-t".to_string(), "12".to_string(),
            "-v".to_string(),
            "--ss".to_string(), "test".to_string(),
            "arg1".to_string(), "arg2".to_string()]).is_ok());
        assert_eq!(3333, port_parameter.get_value());
        assert_eq!(1024 * 1024, max_memory_parameter.get_value());
        assert_eq!(12, threads_parameter.get_value());
        assert_eq!(true, verbose_parameter.get_value());
        assert_eq!("test", string_parameter.get_value());
        assert_eq!(vec!["arg1".to_string(), "arg2".to_string()], arguments.get_other_arguments().clone());
    }
}