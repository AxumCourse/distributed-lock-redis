CREATE TABLE inventory_1 ( -- 库存
	id SERIAL PRIMARY KEY,
	stock INTEGER NOT NULL -- 库存数量
);

-- 插入示例数据
INSERT INTO inventory_1(id, stock) VALUES(1, 5);