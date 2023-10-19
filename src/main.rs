use std::sync::Arc;

use redlock::RedLock;

/// 库存
#[derive(Debug, sqlx::FromRow)]
pub struct Inventory {
    pub id: i32,
    pub stock: i32,
}

#[tokio::main]
async fn main() {
    let pool = get_pool().await.unwrap();
    let pool = Arc::new(pool);

    let mut hs = vec![];

    for i in 0..8 {
        let pool = pool.clone();
        hs.push(tokio::spawn(async move {
            let rl = get_redis_lock(); // 获取锁管理器
            let lc;

            // 尝试获取锁
            loop {
                if let Some(l) = rl
                    .lock(format!("inventory-lock-{}", i).as_bytes(), 1000)
                    .unwrap()
                {
                    lc = l;
                    break;
                }
            }

            // 数据库开启事务
            let mut tx = pool.begin().await.unwrap();

            // 扣减库存，并将扣减后的数据返回
            let inv: Inventory = match sqlx::query_as(
                "UPDATE inventory_1 SET stock = stock-1 WHERE id=1 RETURNING *",
            )
            .fetch_one(&mut *tx)
            .await
            {
                Ok(q) => q,
                Err(e) => {
                    // 回滚事务
                    tx.rollback().await.unwrap();
                    // 释放锁
                    rl.unlock(&lc);
                    eprintln!("{:?}", e);
                    return;
                }
            };

            // 如果扣减后，还有库存
            if inv.stock >= 0 {
                println!("#{} 成功扣减库存，扣减库存【后】 {:?}", i, inv);
            } else {
                tx.rollback().await.unwrap();
                rl.unlock(&lc);
                println!("#{} 库存不足", i);
                return;
            };

            // 提交事务
            tx.commit().await.unwrap();

            // 释放锁
            rl.unlock(&lc);
        }));
    }

    for h in hs {
        let _ = h.await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// 获取数据库连接池
async fn get_pool() -> Result<sqlx::PgPool, sqlx::Error> {
    let dsn = std::env::var("PG_DSN")
        .unwrap_or("postgres://postgres:postgres@127.0.0.1/draft".to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&dsn)
        .await
}

/// 获取 Redis 锁的管理器
fn get_redis_lock() -> RedLock {
    let redis_dsn = std::env::var("REDIS_DSN").unwrap_or("redis://127.0.0.1:6379/".to_string());
    let redis_dsn: Vec<&str> = redis_dsn.split(",").collect();

    RedLock::new(redis_dsn)
}
