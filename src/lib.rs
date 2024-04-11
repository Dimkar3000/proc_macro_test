#![allow(dead_code)]

use core::marker::PhantomData;

use macro_builder::Builder;

pub trait FieldSetter<V> {
    fn set(&mut self, value: V);
}
struct FieldSetterImpl<'a, V, T, const N: usize, F>(&'a mut [T], F, PhantomData<V>);
struct SettersImpl<'a, T, F>(&'a mut [T], F);

impl<'a, V, T, const N: usize, F: Fn(V) -> T> FieldSetter<V> for FieldSetterImpl<'a, V, T, N, F> {
    fn set(&mut self, value: V) {
        self.0[N] = self.1(value);
    }
}

#[derive(Debug, PartialEq, Eq, Builder)]
#[variance(2)]
pub struct Foo {
    pub field1: u16,
    pub field2: u32,
}

#[derive(Debug, PartialEq, Eq, Builder)]
#[variance(3)]
pub struct Bar {
    pub field3: u64,

    #[expand]
    pub foo: Foo,
    pub field4: bool,
}

#[derive(Debug, PartialEq, Eq, Builder)]
#[variance(5)]
pub struct Baz {
    pub field5: u32,
    #[expand]
    pub bar: Bar,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo_observer_works() {
        let mut obs = FooFieldObserver::new();
        obs.setters().field1().set(0);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [&FooFieldType::Field1(0)][..]
        );
        obs.clear_events();

        obs.setters().field1().set(1);
        obs.setters().field2().set(2);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [&FooFieldType::Field1(1), &FooFieldType::Field2(2)][..]
        );
    }

    #[test]
    fn bar_observer_works() {
        let mut obs = BarFieldObserver::new();
        obs.setters().field3().set(0);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [&BarFieldType::Field3(0)][..]
        );
        obs.clear_events();

        obs.setters().field3().set(1);
        obs.setters().foo().field1().set(2);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [
                &BarFieldType::Field3(1),
                &BarFieldType::Foo(FooFieldType::Field1(2))
            ][..]
        );
    }

    #[test]
    fn baz_observer_works() {
        let mut obs = BazFieldObserver::new();
        obs.setters().field5().set(0);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [&BazFieldType::Field5(0)][..]
        );
        obs.clear_events();

        obs.setters().field5().set(1);
        obs.setters().bar().foo().field2().set(2);
        assert_eq!(
            obs.events().collect::<Vec<_>>()[..],
            [
                &BazFieldType::Field5(1),
                &BazFieldType::Bar(BarFieldType::Foo(FooFieldType::Field2(2)))
            ][..]
        );
    }
}
