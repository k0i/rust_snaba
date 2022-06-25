#![allow(dead_code)]
use std::{
    collections::VecDeque,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};

struct PolishNotation<T> {
    stack: VecDeque<T>,
}

impl<T> PolishNotation<T>
where
    T: Mul<Output = T> + Sub<Output = T> + Add<Output = T> + Div<Output = T> + FromStr,
{
    pub fn new(val: T) -> Self {
        Self {
            stack: [val].into(),
        }
    }
    pub fn append<U: ToString>(&mut self, new_val: U) -> Result<Option<T>, &str> {
        if let Ok(int) = new_val.to_string().as_str().parse::<T>() {
            self.push(int);
            Ok(None)
        } else {
            match self.calc(new_val.to_string().as_str()) {
                Ok(res) => Ok(Some(res)),
                Err(e) => Err(e),
            }
        }
    }
    fn calc(&mut self, str: &str) -> Result<T, &str> {
        if self.stack.len() < 2 {
            return Err("stack length smaller than 2. Calculation failed");
        }
        let rhs = self.stack.pop_back().expect("unknown error");
        let lhs = self.stack.pop_back().expect("unknown error");
        match str {
            "+" => Ok(lhs + rhs),
            "*" => Ok(lhs * rhs),
            "-" => Ok(lhs - rhs),
            "/" => Ok(lhs / rhs),
            _ => Err("Operators expected. Calculation failed"),
        }
    }
    fn push(&mut self, new_val: T) {
        self.stack.push_back(new_val);
    }
}

#[cfg(test)]
pub fn nominal_case_int() {
    test_add(1, 2);
    test_sub(3, 2);
    test_div(3, 6);
    test_mul(9, 5);
}

#[cfg(test)]
pub fn nominal_case_float() {
    test_add(1.4, 2.2);
    test_sub(3.5, 2.9);
    test_div(3.322, 6.7890);
    test_mul(9.34849, 5.6534);
}
#[cfg(test)]
fn test_add<T>(lhs: T, rhs: T)
where
    T: Mul<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + Div<Output = T>
        + FromStr
        + ToString
        + std::fmt::Debug
        + PartialEq
        + Copy,
{
    let mut p = PolishNotation::new(lhs);
    let _ = p.append(rhs);
    assert_eq!(p.append("+").unwrap().unwrap(), lhs + rhs);
}

#[cfg(test)]
fn test_sub<T>(lhs: T, rhs: T)
where
    T: Mul<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + Div<Output = T>
        + FromStr
        + ToString
        + std::fmt::Debug
        + PartialEq
        + Copy,
{
    let mut p = PolishNotation::new(lhs);
    let _ = p.append(rhs);
    assert_eq!(p.append("-").unwrap().unwrap(), lhs - rhs);
}
#[cfg(test)]
fn test_div<T>(lhs: T, rhs: T)
where
    T: Mul<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + Div<Output = T>
        + FromStr
        + ToString
        + std::fmt::Debug
        + PartialEq
        + Copy,
{
    let mut p = PolishNotation::new(lhs);
    let _ = p.append(rhs);
    assert_eq!(p.append("/").unwrap().unwrap(), lhs / rhs);
}
#[cfg(test)]
fn test_mul<T>(lhs: T, rhs: T)
where
    T: Mul<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + Div<Output = T>
        + FromStr
        + ToString
        + std::fmt::Debug
        + PartialEq
        + Copy,
{
    let mut p = PolishNotation::new(lhs);
    let _ = p.append(rhs);
    assert_eq!(p.append("*").unwrap().unwrap(), lhs * rhs);
}
