use super::*;

fn session() -> ProjectionSession {
    ProjectionSession("mere:graph".into())
}

fn rows(count: usize, width: usize) -> Vec<QueryRow> {
    (0..count)
        .map(|i| vec![Some(format!("<urn:row:{i}:{}>", "x".repeat(width))), None])
        .collect()
}

fn vars() -> Vec<String> {
    vec!["s".into(), "o".into()]
}

#[test]
fn a_request_and_a_page_round_trip_through_the_carrier_envelope() {
    let request = CarrierRequest {
        id: 9,
        body: CarrierRequestBody::Query(QueryRequest::first(session(), "SELECT * { ?s ?p ?o }")),
    };
    let wire = serde_json::to_vec(&request).unwrap();
    assert_eq!(serde_json::from_slice::<CarrierRequest>(&wire).unwrap(), request);
    let CarrierRequestBody::Query(query) = &request.body else {
        unreachable!()
    };
    let page = QueryResponse::page(query, vars(), &rows(3, 1)).unwrap();
    let reply = CarrierResponse {
        id: 9,
        body: Ok(CarrierResponseBody::Query(Box::new(page))),
    };
    let wire = serde_json::to_vec(&reply).unwrap();
    assert_eq!(serde_json::from_slice::<CarrierResponse>(&wire).unwrap(), reply);
}

#[test]
fn pages_reassemble_the_whole_result_and_the_last_says_so() {
    let all = rows(400, 3000);
    let mut request = QueryRequest::first(session(), "q");
    let mut seen = Vec::new();
    let mut pages = 0;
    loop {
        let page = QueryResponse::page(&request, vars(), &all).unwrap();
        assert_eq!(page.offset as usize, seen.len());
        seen.extend(page.rows.iter().cloned());
        pages += 1;
        match request.next(&page) {
            Some(next) => request = next,
            None => break,
        }
    }
    assert_eq!(seen, all);
    assert!(pages > 1, "400 rows of ~3 KB need more than one 512 KiB page");
}

#[test]
fn a_page_of_a_different_result_is_refused() {
    let first = QueryResponse::page(&QueryRequest::first(session(), "q"), vars(), &rows(400, 3000))
        .unwrap();
    let next = QueryRequest::first(session(), "q").next(&first).unwrap();
    let changed = rows(401, 3000);
    assert_eq!(
        QueryResponse::page(&next, vars(), &changed),
        Err(QueryPageError::ResultChanged)
    );
}

#[test]
fn a_row_larger_than_the_page_is_refused_not_sent_empty() {
    let mut request = QueryRequest::first(session(), "q");
    request.max_bytes = 64;
    assert!(matches!(
        QueryResponse::page(&request, vars(), &rows(1, 100)),
        Err(QueryPageError::RowExceedsPage { row: 0, .. })
    ));
}

#[test]
fn an_empty_result_is_one_page_and_past_the_end_is_refused() {
    let request = QueryRequest::first(session(), "q");
    let page = QueryResponse::page(&request, vars(), &[]).unwrap();
    assert_eq!((page.total_rows, page.next_offset()), (0, None));
    let mut late = request;
    late.offset = 5;
    assert!(matches!(
        QueryResponse::page(&late, vars(), &rows(5, 1)),
        Err(QueryPageError::OffsetPastEnd { offset: 5, total_rows: 5 })
    ));
}

#[test]
fn a_full_page_stays_under_the_native_message_frame() {
    // Quotes and backslashes double under JSON escaping, the worst case for text.
    let all: Vec<QueryRow> = (0..2000)
        .map(|_| vec![Some("\"\\".repeat(200)), Some("\n".repeat(100))])
        .collect();
    let page = QueryResponse::page(&QueryRequest::first(session(), "q"), vars(), &all).unwrap();
    assert!(page.next_offset().is_some(), "the result spans pages");
    let reply = CarrierResponse {
        id: 1,
        body: Ok(CarrierResponseBody::Query(Box::new(page))),
    };
    assert!(serde_json::to_vec(&reply).unwrap().len() < 1024 * 1024);
}

#[test]
fn the_page_size_clamps_and_the_text_bound_holds() {
    let mut request = QueryRequest::first(session(), "q");
    assert_eq!(request.page_bytes(), MAX_QUERY_PAGE_BYTES);
    request.max_bytes = u32::MAX;
    assert_eq!(request.page_bytes(), MAX_QUERY_PAGE_BYTES);
    request.max_bytes = 10;
    assert_eq!(request.page_bytes(), 10);
    assert!(request.text_within_bound());
    request.query = "x".repeat(MAX_QUERY_TEXT_BYTES as usize + 1);
    assert!(!request.text_within_bound());
}

#[test]
fn the_grant_names_the_query_by_its_digest() {
    let a = QueryRequest::first(session(), "SELECT ?s { ?s ?p ?o }");
    let b = QueryRequest::first(session(), "SELECT ?s { ?s ?p ?o } ");
    assert_eq!(a.digest(), ContentHash::of(b"SELECT ?s { ?s ?p ?o }"));
    assert_ne!(a.digest(), b.digest(), "the grant is byte-exact");
}
