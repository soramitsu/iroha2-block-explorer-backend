use std::{
    iter::{Skip, Take},
    num::NonZero,
    ops::Range,
};

use nonzero_ext::nonzero;

type Int = usize;

/// Translate pagination into a range on a list, where first page is 0..n, second is n..2n and so on.
#[derive(Debug, Clone, Copy)]
pub struct DirectPagination {
    page: NonZero<Int>,
    per_page: NonZero<Int>,
    // non-zero because it doesn't make sense to create with zero items
    total_items: NonZero<Int>,
}

impl DirectPagination {
    pub fn new(page: NonZero<Int>, per_page: NonZero<Int>, total_items: NonZero<Int>) -> Self {
        Self {
            page,
            per_page,
            total_items,
        }
    }

    pub fn range(&self) -> Range<Int> {
        let start = (self.page.get() - 1) * self.per_page.get();
        start..(start + self.per_page.get())
    }

    pub fn page(&self) -> NonZero<Int> {
        self.page
    }

    pub fn per_page(&self) -> NonZero<Int> {
        self.per_page
    }

    pub fn total_items(&self) -> NonZero<Int> {
        self.total_items
    }

    pub fn total_pages(&self) -> NonZero<Int> {
        let full = self.total_items.get() / self.per_page.get();
        NonZero::new(if self.total_items.get() % self.per_page.get() > 0 {
            full + 1
        } else {
            full
        })
        .expect("full is always greater than 0")
    }

    pub fn to_limit_offset(&self) -> OffsetLimit {
        let range = self.range();
        OffsetLimit {
            offset: range.start,
            limit: NonZero::new(range.end - range.start).expect("per page is non-zero"),
        }
    }
}

/// Translate pagination into pointers on a list where first page is in the end of the list, and the last one is
/// in the beginning.
///
/// For example, give list with 42 elements and 10 elements per page, pages will point to the following ranges:
///
/// 1. 32..42
/// 2. 22..42
/// 3. 12..22
/// 4. 2..12
/// 5. 0..2
///
/// If page is not specified, the range will be 0..12, i.e. latest full page + latest pending page
#[derive(Debug, Clone, Copy)]
pub struct ReversePagination {
    len: NonZero<Int>,
    per_page: NonZero<Int>,
    page: Option<NonZero<Int>>,
    total_pages: NonZero<Int>,
}

impl ReversePagination {
    /// Compute pagination for provided list's `len`, items `per_page`, and optional `page` number (starting from 1).
    pub fn new(
        total_items: NonZero<Int>,
        per_page: NonZero<Int>,
        page: Option<NonZero<Int>>,
    ) -> Result<Self, ReversePaginationError> {
        let full_pages = total_items.get() / per_page.get();
        let total_pages = if total_items.get() % per_page.get() > 0 {
            NonZero::new(full_pages + 1).expect("is at least 1")
        } else {
            NonZero::new(full_pages).unwrap_or(nonzero!(1usize))
        };

        if let Some(page) = &page {
            if page.get() > total_pages.get() {
                return Err(ReversePaginationError::PageOutOfBounds {
                    page: page.get(),
                    max: total_pages.get(),
                });
            }
        }

        Ok(Self {
            len: total_items,
            per_page,
            page,
            total_pages,
        })
    }

    /// Indices range in the list this pagination translates into
    pub fn range(&self) -> Range<Int> {
        let (len, per_page) = (self.len.get(), self.per_page.get());

        let (start, end) = if let Some(page) = self.page {
            let page = page.get();
            let start = per_page * (page - 1);
            (start, (start + per_page).min(len))
        } else {
            let full_pages = len / per_page;
            let start = (full_pages.max(1) - 1) * per_page;
            (start, len)
        };

        (len - end)..(len - start)
    }

    /// Total available pages of data, including pending one.
    pub fn total_pages(&self) -> NonZero<Int> {
        self.total_pages
    }

    /// Page number, initially given or computed
    pub fn page(&self) -> NonZero<Int> {
        self.page.unwrap_or(self.total_pages)
    }

    /// Length of the list
    pub fn total_items(&self) -> NonZero<Int> {
        self.len
    }

    /// Items per page
    pub fn per_page(&self) -> NonZero<Int> {
        self.per_page
    }

    /// Translate to [`OffsetLimit`] applicable for a reverse iteration
    pub fn to_offset_limit_for_rev_iter(&self) -> OffsetLimit {
        let range = self.range();
        OffsetLimit {
            offset: range.start,
            limit: NonZero::new(range.end - range.start).expect("per_page is non-zero"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReversePaginationError {
    #[error("page is out of bounds: maximum allowed is {max}, got {page}")]
    PageOutOfBounds { page: Int, max: Int },
}

/// Specification of offset + limit. Translates [`DirectPagination`] and [`ReversePagination`] into [`Iterator::skip`] and [`Iterator::take`].
pub struct OffsetLimit {
    pub offset: usize,
    pub limit: NonZero<usize>,
}

pub trait OffsetLimitIteratorExt: Iterator {
    /// Apply offset and limit to the iterator
    fn offset_limit(self, value: OffsetLimit) -> Take<Skip<Self>>
    where
        Self: Sized,
    {
        self.skip(value.offset).take(value.limit.get())
    }
}

impl<I> OffsetLimitIteratorExt for I where I: Iterator {}

#[cfg(test)]
mod test {
    use super::*;

    mod reverse_pagination {
        use super::*;

        macro_rules! assert_case {
            (($count:expr, $per_page:expr, $page:expr) => ($range:expr, $total_pages:expr)) => {
                let x = ReversePagination::new(
                    NonZero::new($count).unwrap(),
                    NonZero::new($per_page).unwrap(),
                    $page.map(|num| NonZero::new(num).unwrap()),
                )
                .expect("should compute fine");
                assert_eq!(x.range(), $range, "bad range for {x:?}");
                assert_eq!(
                    x.total_pages(),
                    NonZero::new($total_pages).unwrap(),
                    "bad total for {x:?}"
                );
            };
        }

        #[test]
        fn computing_reverse_pagination() {
            assert_case!((100, 10, None) => (0..10, 10));
            assert_case!((105, 10, None) => (0..15, 11));
            assert_case!((23, 10, Some(1)) => (13..23, 3));
            assert_case!((23, 10, Some(2)) => (3..13, 3));
            assert_case!((23, 10, None) => (0..13, 3));
            assert_case!((20, 10, Some(1)) => (10..20, 2));
            assert_case!((20, 10, Some(2)) => (0..10, 2));
            assert_case!((43, 10, Some(1)) => (33..43, 5));
            assert_case!((43, 10, Some(2)) => (23..33, 5));
            assert_case!((5, 10, None) => (0..5, 1));
            assert_case!((10, 10, None) => (0..10, 1));
            assert_case!((11, 10, None) => (0..11, 2));
            assert_case!((5, 2, Some(2)) => (1..3, 3));
            assert_case!((5, 2, Some(3)) => (0..1, 3));
            assert_case!((106, 10, Some(11)) => (0..6, 11));
        }

        #[test]
        fn page_out_of_bounds() {
            let ReversePaginationError::PageOutOfBounds { page, max } = ReversePagination::new(
                nonzero!(23usize),
                nonzero!(10usize),
                Some(nonzero!(4usize)),
            )
            .expect_err("should be out of bounds");
            assert_eq!(page, 4);
            assert_eq!(max, 3);
        }

        #[test]
        fn computed_page() {
            let value = ReversePagination::new(nonzero!(15usize), nonzero!(10usize), None).unwrap();
            assert_eq!(value.page(), nonzero!(2usize));

            let value = ReversePagination::new(
                nonzero!(15usize),
                nonzero!(10usize),
                Some(nonzero!(2usize)),
            )
            .unwrap();
            assert_eq!(value.page(), nonzero!(2usize));

            let value = ReversePagination::new(
                nonzero!(15usize),
                nonzero!(10usize),
                Some(nonzero!(1usize)),
            )
            .unwrap();
            assert_eq!(value.page(), nonzero!(1usize));
        }
    }

    #[test]
    fn create_direct_pagination() {
        let value = DirectPagination::new(nonzero!(4usize), nonzero!(10usize), nonzero!(95usize));
        assert_eq!(value.range(), 30..40);
    }
}
