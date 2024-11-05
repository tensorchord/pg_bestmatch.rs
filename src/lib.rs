mod tokenizer;

pgrx::pg_module_magic!();
pgrx::extension_sql_file!("./sql/finalize.sql", finalize);

#[cfg(not(all(target_endian = "little", target_pointer_width = "64")))]
compile_error!("Target is not supported.");

#[cfg(not(any(
    feature = "pg12",
    feature = "pg13",
    feature = "pg14",
    feature = "pg15",
    feature = "pg16",
    feature = "pg17"
)))]
compile_error!("PostgreSQL version must be selected.");

#[allow(non_snake_case)]
#[pgrx::pg_guard]
unsafe extern "C" fn _PG_init() {}

#[pgrx::pg_extern(immutable, strict, parallel_safe)]
pub fn tokenize(t: &str, tokenizer: &str, model: Option<&str>) -> Vec<String> {
    tokenizer::tokenize(tokenizer, model, t)
}

#[derive(Debug)]
#[repr(C)]
struct RecordMat {
    token: [u8; pgrx::pg_sys::NAMEDATALEN as usize],
    id: i32,
    how_many_tokens: i32,
    idf: f32,
}

#[allow(clippy::too_many_arguments)]
#[pgrx::pg_extern(strict, parallel_safe)]
pub fn bm25_document_to_svector_internal(
    mat: pgrx::pg_sys::Oid,
    idx: pgrx::pg_sys::Oid,
    b: f32,
    k1: f32,
    words: i32,
    docs: i32,
    dims: i32,
    t: &str,
    style: &str,
    tokenizer: &str,
    model: Option<&str>,
) -> String {
    use std::collections::BTreeMap;
    let tokens = tokenize(t, tokenizer, model);
    let mut x = BTreeMap::<u32, u32>::new();
    unsafe {
        use pgrx::pg_sys::*;
        use std::collections::btree_map::Entry;
        use std::ffi::CString;
        let heap = table_open(mat, AccessShareLock as _);
        let index = index_open(idx, AccessShareLock as _);
        let slot = MakeSingleTupleTableSlot((*heap).rd_att, table_slot_callbacks(heap));
        let scan = index_beginscan(heap, index, GetActiveSnapshot(), 1, 0);
        for token in tokens.iter() {
            if let Ok(token) = CString::new(token.as_str()) {
                let mut key = std::mem::zeroed::<ScanKeyData>();
                pgrx::pg_sys::ScanKeyInit(
                    &mut key,
                    /* attr 1 */ 1,
                    pgrx::pg_sys::BTEqualStrategyNumber as _,
                    pgrx::pg_sys::F_NAMEEQ.into(),
                    token.as_ptr().into(),
                );
                index_rescan(scan, &mut key, 1, std::ptr::null_mut(), 0);
                if index_getnext_slot(scan, ScanDirection::ForwardScanDirection, slot) {
                    let mut should_free = false;
                    let tuple = ExecFetchSlotHeapTuple(slot, false, &mut should_free);
                    debug_assert!(!tuple.is_null());
                    let row = (*tuple)
                        .t_data
                        .cast::<u8>()
                        .add((*(*tuple).t_data).t_hoff as _)
                        .cast::<RecordMat>();
                    let id = (*row).id as u32;
                    match x.entry(id) {
                        Entry::Vacant(e) => {
                            e.insert(1);
                        }
                        Entry::Occupied(mut e) => {
                            *e.get_mut() += 1;
                        }
                    }
                    if should_free {
                        pfree(tuple.cast());
                    }
                }
            }
        }
        index_endscan(scan);
        ExecDropSingleTupleTableSlot(slot);
        index_close(index, AccessShareLock as _);
        table_close(heap, AccessShareLock as _);
    }
    match style {
        "pgvecto.rs" => {
            let avgdl = words as f32 / docs as f32;
            let length = x.values().sum::<u32>() as f32;
            let mut result = "{".to_string();
            for (index, value) in x.into_iter() {
                let value = value as f32 / (value as f32 + k1 * ((1.0 - b) + b * (length / avgdl)));
                result.push_str(&format!("{}:{value}, ", index));
            }
            if result.ends_with(", ") {
                result.pop();
                result.pop();
            }
            result.push('}');
            result.push('/');
            result.push_str(&dims.to_string());
            result
        }
        "pgvector" => {
            let avgdl = words as f32 / docs as f32;
            let length = x.values().sum::<u32>() as f32;
            let mut result = "{".to_string();
            for (index, value) in x.into_iter() {
                let value = value as f32 / (value as f32 + k1 * ((1.0 - b) + b * (length / avgdl)));
                result.push_str(&format!("{}:{value}, ", index + 1));
            }
            if result.ends_with(", ") {
                result.pop();
                result.pop();
            }
            result.push('}');
            result.push('/');
            result.push_str(&dims.to_string());
            result
        }
        _ => pgrx::error!("unknown svector style: {}", style),
    }
}

#[pgrx::pg_extern(strict, parallel_safe)]
pub fn bm25_query_to_svector_internal(
    mat: pgrx::pg_sys::Oid,
    idx: pgrx::pg_sys::Oid,
    dims: i32,
    t: &str,
    style: &str,
    tokenizer: &str,
    model: Option<&str>,
) -> String {
    use std::collections::BTreeMap;
    let tokens = tokenize(t, tokenizer, model);
    let mut x = BTreeMap::<u32, f32>::new();
    unsafe {
        use pgrx::pg_sys::*;
        use std::ffi::CString;
        let heap = table_open(mat, AccessShareLock as _);
        let index = index_open(idx, AccessShareLock as _);
        let slot = MakeSingleTupleTableSlot((*heap).rd_att, table_slot_callbacks(heap));
        let scan = index_beginscan(heap, index, GetActiveSnapshot(), 1, 0);
        for token in tokens.iter() {
            if let Ok(token) = CString::new(token.as_str()) {
                let mut key = std::mem::zeroed::<ScanKeyData>();
                pgrx::pg_sys::ScanKeyInit(
                    &mut key,
                    /* attr 1 */ 1,
                    pgrx::pg_sys::BTEqualStrategyNumber as _,
                    pgrx::pg_sys::F_NAMEEQ.into(),
                    token.as_ptr().into(),
                );
                index_rescan(scan, &mut key, 1, std::ptr::null_mut(), 0);
                if index_getnext_slot(scan, ScanDirection::ForwardScanDirection, slot) {
                    let mut should_free = false;
                    let tuple = ExecFetchSlotHeapTuple(slot, false, &mut should_free);
                    debug_assert!(!tuple.is_null());
                    let row = (*tuple)
                        .t_data
                        .cast::<u8>()
                        .add((*(*tuple).t_data).t_hoff as _)
                        .cast::<RecordMat>();
                    let id = (*row).id as u32;
                    let idf = (*row).idf;
                    x.insert(id, idf);
                    if should_free {
                        pfree(tuple.cast());
                    }
                }
            }
        }
        index_endscan(scan);
        ExecDropSingleTupleTableSlot(slot);
        index_close(index, AccessShareLock as _);
        table_close(heap, AccessShareLock as _);
    }
    match style {
        "pgvecto.rs" => {
            // https://github.com/pinecone-io/pinecone-text/issues/69
            let sum = x.values().copied().sum::<f32>();
            let mut result = "{".to_string();
            for (index, value) in x.into_iter() {
                result.push_str(&format!("{}:{}, ", index, value / sum));
            }
            if result.ends_with(", ") {
                result.pop();
                result.pop();
            }
            result.push('}');
            result.push('/');
            result.push_str(&dims.to_string());
            result
        }
        "pgvector" => {
            // https://github.com/pinecone-io/pinecone-text/issues/69
            let sum = x.values().copied().sum::<f32>();
            let mut result = "{".to_string();
            for (index, value) in x.into_iter() {
                result.push_str(&format!("{}:{}, ", index + 1, value / sum));
            }
            if result.ends_with(", ") {
                result.pop();
                result.pop();
            }
            result.push('}');
            result.push('/');
            result.push_str(&dims.to_string());
            result
        }
        _ => pgrx::error!("unknown svector style: {}", style),
    }
}
