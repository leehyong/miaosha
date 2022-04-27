use miaosha;

alter table shopping_cart add column is_stock tinyint not null default 1
    comment '是否有货:0,无货;1, 有货' after `status`;