use super::*;
use color_eyre::eyre::{eyre, Context, Result};
use iroha_client::client::PaginatedQueryOutput;
use iroha_data_model::prelude::{Pagination as IrohaPagination, Query, QueryBox, Value};
use serde::Deserialize;

/// Represents some items list with its pagination data
#[derive(Serialize)]
pub struct Paginated<T> {
    pagination: PaginationDTO,
    data: T,
}

impl<T> Paginated<T> {
    /// Wraps some items list with a provided pagination data
    pub fn new(data: T, pagination: PaginationDTO) -> Self {
        Self { data, pagination }
    }

    pub fn map<U, F>(self, f: F) -> Paginated<U>
    where
        F: FnOnce(T) -> U,
    {
        Paginated::new(f(self.data), self.pagination)
    }
}

impl<R> TryFrom<PaginatedQueryOutput<R>> for Paginated<R::Output>
where
    R: Query + Into<QueryBox> + Debug,
    <R::Output as TryFrom<Value>>::Error: Into<color_eyre::eyre::Error>,
{
    type Error = color_eyre::Report;

    fn try_from(
        PaginatedQueryOutput {
            output,
            pagination,
            total,
        }: PaginatedQueryOutput<R>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            pagination: PaginationDTO::try_from(IrohaPaginationWithTotal { pagination, total })?,
            data: output,
        })
    }
}

/// Pagination data returned to web
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PaginationDTO {
    /// Current page. Starts from 1.
    pub page: Positive,
    /// Represents pagination scale
    pub page_size: Positive,
    /// Total count of paginated items
    pub total: u64,
}

impl PaginationDTO {
    pub fn from_unchecked_nums(page: u32, page_size: u32, total: u64) -> Result<Self> {
        Ok(Self {
            page: page.try_into().wrap_err("Failed to make page")?,
            page_size: page_size.try_into().wrap_err("Failed to make page size")?,
            total,
        })
    }
}

/// [`IrohaPagination`] doesn't store a `total` amount of records. This struct does.
struct IrohaPaginationWithTotal {
    pagination: IrohaPagination,
    total: u64,
}

/// # Errors
/// Fails if [`IrohaPagination`] has data that is not aligned to pages.
/// For example, if there is a `limit = 10`, but `start = 5`, it means that we have a page size = 10,
/// but there is no a first page. Or if there is no limit, but start not equals to zero -
/// which page size do we have?
impl TryFrom<IrohaPaginationWithTotal> for PaginationDTO {
    type Error = color_eyre::Report;

    fn try_from(
        IrohaPaginationWithTotal {
            pagination: IrohaPagination { start, limit },
            total,
        }: IrohaPaginationWithTotal,
    ) -> Result<Self, Self::Error> {
        match start {
            None => {
                let page = 1;
                let page_size = match limit {
                    None => total.try_into()?,
                    Some(limit) => limit,
                };

                Self::from_unchecked_nums(page, page_size, total)
            }
            Some(start) => match limit {
                None => {
                    let (page, page_size) = if total % (start as u64) == 0 {
                        (2, start)
                    } else {
                        return Err(eyre!(
                            "`start` ({start}) is not aligned with `total` ({total})"
                        ));
                    };

                    Self::from_unchecked_nums(page, page_size, total)
                }
                Some(limit) => {
                    let (page, page_size) = if start % limit == 0 {
                        ((start / limit) + 1, limit)
                    } else {
                        return Err(eyre!(
                            "`start` ({start}) is not aligned with `limit` ({limit})"
                        ));
                    };

                    Self::from_unchecked_nums(page, page_size, total)
                }
            },
        }
    }
}

mod positive_int {
    use color_eyre::eyre::{eyre, Report, Result};
    use serde::{de, Serialize};
    use std::fmt;

    /// Could only contain a positive (non-zero) integer
    #[derive(Serialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub struct Positive(u32);

    impl Positive {
        pub fn value(self) -> u32 {
            self.0
        }

        pub const fn try_from_const(value: u32) -> Result<Self, &'static str> {
            if value > 0 {
                Ok(Self(value))
            } else {
                Err("zero is not allowed")
            }
        }
    }

    impl TryFrom<u32> for Positive {
        type Error = Report;

        fn try_from(value: u32) -> Result<Self, Self::Error> {
            Self::try_from_const(value).map_err(|s| eyre!("{}", s))
        }
    }

    impl<'de> de::Deserialize<'de> for Positive {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> de::Visitor<'de> for Visitor {
                type Value = Positive;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a positive integer")
                }

                fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Positive::try_from(v).map_err(|_err| {
                        de::Error::invalid_value(de::Unexpected::Unsigned(v as u64), &self)
                    })
                }
            }

            deserializer.deserialize_u32(Visitor)
        }
    }
}

pub use positive_int::Positive;

#[derive(Deserialize)]
pub struct PaginationQueryParams {
    #[serde(default = "default_page")]
    pub page: Positive,
    #[serde(default = "default_page_size")]
    pub page_size: Positive,
}

pub const DEFAULT_PAGE: Positive = match Positive::try_from_const(1) {
    Ok(v) => v,
    Err(e) => panic!("{}", e),
};

pub const DEFAULT_PAGE_SIZE: Positive = match Positive::try_from_const(15) {
    Ok(v) => v,
    Err(e) => panic!("{}", e),
};

const fn default_page() -> Positive {
    DEFAULT_PAGE
}

const fn default_page_size() -> Positive {
    DEFAULT_PAGE_SIZE
}

impl From<PaginationQueryParams> for IrohaPagination {
    fn from(PaginationQueryParams { page_size, page }: PaginationQueryParams) -> Self {
        let page = page.value();
        let page_size = page_size.value();
        Self::new(Some((page - 1) * page_size), Some(page_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_query_into_iroha_pagination() {
        let params = PaginationQueryParams {
            page: 3.try_into().unwrap(),
            page_size: 12.try_into().unwrap(),
        };

        let mapped: IrohaPagination = params.into();

        assert_eq!(mapped.start, Some(24));
        assert_eq!(mapped.limit, Some(12));
    }

    mod iroha_pagination_conversion {
        use super::*;

        #[test]
        fn from_valid_complete_data() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(Some(15), Some(5)),
                total: 50,
            };

            let result = PaginationDTO::try_from(pagination).unwrap();

            assert_eq!(result.page.value(), 4);
            assert_eq!(result.page_size.value(), 5);
            assert_eq!(result.total, 50)
        }

        #[test]
        fn from_invalid_complete_data() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(Some(2), Some(5)),
                total: 50,
            };

            let _err = PaginationDTO::try_from(pagination).unwrap_err();
        }

        #[test]
        fn no_start() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(None, Some(5)),
                total: 25,
            };

            let result = PaginationDTO::try_from(pagination).unwrap();

            assert_eq!(result.page.value(), 1);
            assert_eq!(result.page_size.value(), 5);
            assert_eq!(result.total, 25);
        }

        #[test]
        fn no_limit_but_start_fits_to_total() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(Some(13), None),
                total: 26,
            };

            let result = PaginationDTO::try_from(pagination).unwrap();

            assert_eq!(result.page.value(), 2);
            assert_eq!(result.page_size.value(), 13);
            assert_eq!(result.total, 26)
        }

        #[test]
        fn no_limit_but_start_fits_to_total_5_times() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(Some(10), None),
                total: 50,
            };

            let result = PaginationDTO::try_from(pagination).unwrap();

            assert_eq!(result.page.value(), 2);
            assert_eq!(result.page_size.value(), 10);
            assert_eq!(result.total, 50)
        }

        #[test]
        fn no_limit_and_start_doesnt_fit_to_total() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(Some(5), None),
                total: 26,
            };

            let _err = PaginationDTO::try_from(pagination).unwrap_err();
        }

        #[test]
        fn no_limit_and_start() {
            let pagination = IrohaPaginationWithTotal {
                pagination: IrohaPagination::new(None, None),
                total: 10,
            };

            let result = PaginationDTO::try_from(pagination).unwrap();

            assert_eq!(result.page.value(), 1);
            assert_eq!(result.page_size.value(), 10);
            assert_eq!(result.total, 10)
        }
    }
}
