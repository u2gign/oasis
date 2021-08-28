use super::query::Query;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::SqliteRow;
use sqlx::{Connection, FromRow, Sqlite, Transaction};

pub async fn fetch_single<'r, T>(
    query: Query<'r>,
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<Option<T>>
where
    T: Send + Unpin + for<'a> FromRow<'a, SqliteRow>,
{
    let stmt = prepare_sql(query.sql, &query.args);
    Ok(stmt.fetch_optional(conn).await?)
}

pub async fn fetch_multiple<'r, T>(
    query: Query<'r>,
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<Vec<T>>
where
    T: Send + Unpin + for<'a> FromRow<'a, SqliteRow>,
{
    let stmt = prepare_sql(query.sql, &query.args);
    Ok(stmt.fetch_all(conn).await?)
}

pub async fn tx_execute_2<'r>(
    queries: Vec<Query<'r>>,
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<()> {
    let mut tx = conn.begin().await?;

    for query in queries.iter() {
        let stmt = prepare_exec_sql(query.sql, &query.args);
        stmt.execute(&mut tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn tx_execute<'r>(
    query: Query<'r>,
    tx: &mut Transaction<'_, Sqlite>,
) -> anyhow::Result<i64> {
    let mut insert_id = -1;
    let stmt = prepare_exec_sql(query.sql, &query.args);
    if query.sql.to_lowercase().starts_with("insert") {
        insert_id = stmt.execute(&mut *tx).await?.last_insert_rowid();
    }

    Ok(insert_id)
}

pub async fn execute<'a>(
    query: Query<'a>,
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<()> {
    let stmt = prepare_exec_sql(query.sql, &query.args);
    stmt.execute(conn).await?;

    Ok(())
}

pub async fn insert_single<'a>(
    query: Query<'a>,
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<i64> {
    let stmt = prepare_exec_sql(query.sql, &query.args);
    let id = stmt.execute(conn).await?.last_insert_rowid();

    Ok(id)
}

fn prepare_sql<'a, T>(
    sql: &'a str,
    args: &'a Vec<String>,
) -> sqlx::query::QueryAs<'a, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'a>>
where
    T: Send + Unpin + for<'b> FromRow<'b, SqliteRow>,
{
    let mut stmt = sqlx::query_as(sql);
    for arg in args.iter() {
        stmt = stmt.bind(arg);
    }

    stmt
}

fn prepare_exec_sql<'a>(
    sql: &'a str,
    args: &'a Vec<String>,
) -> sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>> {
    let mut stmt = sqlx::query(sql);
    for arg in args.iter() {
        stmt = stmt.bind(arg);
    }

    stmt
}
