//! A SPARQL query over the dataset a session's endpoint holds
//! (graphshell:mere-cli D4, D6, D7). Appended to the carrier the way
//! retained pages were: older peers never send it, and an endpoint that holds
//! no dataset refuses it.

use super::*;

/// Most bytes of query text one request carries (D6).
pub const MAX_QUERY_TEXT_BYTES: u32 = 64 * 1024;

/// Most bytes of encoded rows one reply carries (D6). Sized like
/// [`MAX_RESOURCE_CHUNK_BYTES`]: JSON escaping of row text can grow it, and the
/// rest of the frame still fits under the 1 MiB carrier cap.
pub const MAX_QUERY_PAGE_BYTES: u32 = 512 * 1024;

/// One solution: the bound term per variable in display form, `None` where
/// the variable is unbound.
pub type QueryRow = Vec<Option<String>>;

/// Run `query`, or read a later page of its result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub session: ProjectionSession,
    pub query: String,
    /// The first row wanted; `0` starts the result.
    pub offset: u64,
    /// The result earlier pages came from, `None` on the first page. An
    /// endpoint whose result no longer has this address refuses, so a client
    /// never stitches pages of two different results together.
    pub result: Option<ContentHash>,
    /// Most bytes of rows this client accepts back. `0` lets the endpoint
    /// choose; any value is clamped to [`MAX_QUERY_PAGE_BYTES`].
    pub max_bytes: u32,
}

impl QueryRequest {
    /// The first page of `query`.
    pub fn first(session: ProjectionSession, query: impl Into<String>) -> Self {
        Self {
            session,
            query: query.into(),
            offset: 0,
            result: None,
            max_bytes: 0,
        }
    }

    /// The page after `previous`, or `None` when it was the last.
    pub fn next(&self, previous: &QueryResponse) -> Option<Self> {
        previous.next_offset().map(|offset| Self {
            offset,
            result: Some(previous.result),
            ..self.clone()
        })
    }

    /// What an invitation's scope names to allow this query (D7).
    pub fn digest(&self) -> ContentHash {
        ContentHash::of(self.query.as_bytes())
    }

    pub fn text_within_bound(&self) -> bool {
        self.query.len() <= MAX_QUERY_TEXT_BYTES as usize
    }

    /// The page size the endpoint applies.
    pub fn page_bytes(&self) -> u32 {
        match self.max_bytes {
            0 => MAX_QUERY_PAGE_BYTES,
            asked => asked.min(MAX_QUERY_PAGE_BYTES),
        }
    }
}

/// One page of a query's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResponse {
    pub session: ProjectionSession,
    pub variables: Vec<String>,
    pub rows: Vec<QueryRow>,
    pub offset: u64,
    /// Rows in the whole result, so a client knows when it is done.
    pub total_rows: u64,
    /// The address of the whole result ([`QueryResponse::address`]).
    pub result: ContentHash,
}

/// Why an endpoint cannot cut the page asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryPageError {
    /// The request named a result this one is not.
    ResultChanged,
    /// The offset is past the last row.
    OffsetPastEnd { offset: u64, total_rows: u64 },
    /// One row alone is larger than the page; an empty page would strand the
    /// client below the end, so none is sent.
    RowExceedsPage { row: u64, bytes: usize, page: u32 },
}

impl fmt::Display for QueryPageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResultChanged => f.write_str("the result changed since the earlier pages"),
            Self::OffsetPastEnd { offset, total_rows } => {
                write!(f, "offset {offset} is past the result's {total_rows} rows")
            },
            Self::RowExceedsPage { row, bytes, page } => {
                write!(f, "row {row} encodes to {bytes} bytes, over the {page}-byte page")
            },
        }
    }
}

impl std::error::Error for QueryPageError {}

impl QueryResponse {
    /// The address of a whole result: BLAKE3 over its JSON encoding.
    pub fn address(variables: &[String], rows: &[QueryRow]) -> ContentHash {
        let encoded = serde_json::to_vec(&(variables, rows)).expect("strings always encode");
        ContentHash::of(&encoded)
    }

    /// Cut the page `request` asks for out of the whole result.
    ///
    /// # Errors
    ///
    /// The request names a different result, starts past the end, or the
    /// first row it would carry does not fit a page.
    pub fn page(
        request: &QueryRequest,
        variables: Vec<String>,
        rows: &[QueryRow],
    ) -> Result<Self, QueryPageError> {
        let result = Self::address(&variables, rows);
        if request.result.is_some_and(|earlier| earlier != result) {
            return Err(QueryPageError::ResultChanged);
        }
        let total_rows = rows.len() as u64;
        let start = usize::try_from(request.offset)
            .ok()
            .filter(|start| *start < rows.len() || (*start == 0 && rows.is_empty()))
            .ok_or(QueryPageError::OffsetPastEnd {
                offset: request.offset,
                total_rows,
            })?;
        let page = request.page_bytes();
        let mut used = 0usize;
        let mut taken = Vec::new();
        for (index, row) in rows[start..].iter().enumerate() {
            let bytes = serde_json::to_vec(row).expect("strings always encode").len() + 1;
            if used + bytes > page as usize {
                if taken.is_empty() {
                    return Err(QueryPageError::RowExceedsPage {
                        row: (start + index) as u64,
                        bytes,
                        page,
                    });
                }
                break;
            }
            used += bytes;
            taken.push(row.clone());
        }
        Ok(Self {
            session: request.session.clone(),
            variables,
            rows: taken,
            offset: request.offset,
            total_rows,
            result,
        })
    }

    /// Where the next page starts, or `None` after the last.
    pub fn next_offset(&self) -> Option<u64> {
        let end = self.offset + self.rows.len() as u64;
        (end < self.total_rows).then_some(end)
    }
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
