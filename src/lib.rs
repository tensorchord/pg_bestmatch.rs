pgrx::pg_module_magic!();
pgrx::extension_sql_file!("./sql/bootstrap.sql", bootstrap);
pgrx::extension_sql_file!("./sql/finalize.sql", finalize);

#[cfg(not(all(target_endian = "little", target_pointer_width = "64")))]
compile_error!("Target is not supported.");

#[cfg(not(any(
    feature = "pg12",
    feature = "pg13",
    feature = "pg14",
    feature = "pg15",
    feature = "pg16"
)))]
compiler_error!("PostgreSQL version must be selected.");

#[pgrx::pg_guard]
unsafe extern "C" fn _PG_init() {}
