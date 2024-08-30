use crate::repo::{ListAccountsParams, ListDomainParams, ListTransactionsParams};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Database, Decode, Encode, QueryBuilder, Sqlite, Type};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Range;
use std::str::FromStr;

pub trait QueryBuilderExt<'a> {
    fn push_custom<T>(&mut self, value: T) -> &mut Self
    where
        T: PushCustom<'a>;
}

impl<'a> QueryBuilderExt<'a> for QueryBuilder<'a, Sqlite> {
    fn push_custom<T>(&mut self, value: T) -> &mut Self
    where
        T: PushCustom<'a>,
    {
        value.push_custom(self);
        self
    }
}

pub trait PushCustom<'a>
where
    Self: 'a,
{
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>);
}

pub struct PushFn<'a, F>(F, PhantomData<&'a ()>)
where
    F: FnOnce(&mut QueryBuilder<'a, Sqlite>) + 'a;

pub fn push_fn<'a, F>(f: F) -> PushFn<'a, F>
where
    F: FnOnce(&mut QueryBuilder<'a, Sqlite>) + 'a,
{
    PushFn(f, PhantomData::default())
}

impl<'a, F> PushCustom<'a> for PushFn<'a, F>
where
    F: FnOnce(&mut QueryBuilder<'a, Sqlite>) + 'a,
{
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        self.0(builder);
    }
}

pub struct PushDisplay<T>(pub T);

impl<'a, T: Display + 'a> PushCustom<'a> for PushDisplay<T> {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder.push(self.0);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AsText<T>(pub T);

impl<T, D: Database> Type<D> for AsText<T>
where
    String: Type<D>,
{
    fn type_info() -> <D as Database>::TypeInfo {
        <String as Type<D>>::type_info()
    }
}

impl<'r, T> Decode<'r, Sqlite> for AsText<T>
where
    T: FromStr,
    <T as FromStr>::Err: StdError + Send + Sync + 'static,
{
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> std::result::Result<Self, BoxDynError> {
        let value = <&str as Decode<'_, Sqlite>>::decode(value)?;
        let id = value.parse()?;
        Ok(Self(id))
    }
}

impl<'q, T> Encode<'q, Sqlite> for AsText<T>
where
    T: Display,
{
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> std::result::Result<IsNull, BoxDynError> {
        buf.push(SqliteArgumentValue::Text(Cow::Owned(format!("{}", self.0))));
        Ok(IsNull::No)
    }
}

#[derive(Debug)]
pub struct LimitOffset {
    limit: u64,
    offset: u64,
}

impl PushCustom<'_> for LimitOffset {
    fn push_custom(self, builder: &mut QueryBuilder<'_, Sqlite>) {
        builder
            .push(" limit ")
            .push_bind(self.limit as u32)
            .push(" offset ")
            .push_bind(self.offset as u32);
    }
}

impl From<Range<u64>> for LimitOffset {
    fn from(Range { start, end }: Range<u64>) -> Self {
        Self {
            offset: start,
            limit: end - start,
        }
    }
}
