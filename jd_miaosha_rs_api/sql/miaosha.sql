create database if not exists  miaosha ;
use miaosha;

create table if not exists miaosha.user(
    id int unsigned PRIMARY KEY AUTO_INCREMENT,
    name varchar(64) not null default '',
    mac_addr varchar(80) not null default '' comment '最新登录的mac地址',
    vip_level tinyint unsigned not null default 0 comment 'vip等级。 0,不是vip;1及以上才是vip。',
    activate_code char(32) not null default '' comment '激活码',
    expire_dt datetime comment '过期时间',
    create_time datetime not null,
    update_time datetime not null,
    index ix_activatecode(`activate_code`)
)DEFAULT CHARSET=utf8mb4;

create table if not exists miaosha.login_history(
    id int unsigned PRIMARY KEY AUTO_INCREMENT,
    user_id int unsigned not null comment '用户id。为了性能考虑这里不创建外键关联用户表',
    mac_addr varchar(80) not null default '' comment '每次登录时的mac地址',
    create_time datetime not null,
    index ix_userid(`user_id`)
)DEFAULT CHARSET=utf8mb4;

create table if not exists miaosha.vip_history(
  id int unsigned PRIMARY KEY AUTO_INCREMENT,
  user_id int unsigned not null comment '用户id。为了性能考虑这里不创建外键关联用户表',
  vip_level tinyint unsigned not null default 0 comment 'vip等级。 0,不是vip;1及以上才是vip。',
  expire_dt datetime not null comment '过期时间',
  start_dt datetime not null comment '生效开始时间',
  create_time datetime not null,
  index ix_userid(`user_id`)
)DEFAULT CHARSET=utf8mb4;

create table if not exists miaosha.platform_account(
      id int unsigned PRIMARY KEY AUTO_INCREMENT,
      user_id int unsigned not null comment '用户id。为了性能考虑这里不创建外键关联用户表',
      account varchar(512) not null  default '' comment '账户',
      platform tinyint unsigned not null default '0' comment '用户对应的平台。0: 京东',
      pwd varchar(512) not null default '' comment '加密后密码',
      eid varchar(512) not null default '' comment 'eid',
      fp  varchar(512) not null default '' comment 'fp',
      cookie text comment 'cookie',
      cookie_last_update_dt datetime comment 'cookie的最新更新时间',
      create_time datetime not null,
      update_time datetime not null,
      unique ux_userid_account(`user_id`, `account`, `platform`)
)DEFAULT CHARSET=utf8mb4;

create table if not exists miaosha.shopping_cart(
    id int unsigned PRIMARY KEY AUTO_INCREMENT,
    user_id int not null comment '用户id。为了性能考虑这里不创建外键关联用户表',
    platform tinyint unsigned not null default '0' comment '商品对应的平台。0: 京东',
    category tinyint unsigned not null default '0' comment '商品分类。0: 电脑',
    sku varchar(40) not null comment '商品sku',
    name varchar(1024) not null comment '商品名称',
    purchase_num int unsigned not null default 0 comment '购买数量',
    yuyue_dt datetime comment '预约购买时间',
    yuyue_start_dt datetime comment '预约开始时间',
    yuyue_end_dt datetime comment '预约结束购买时间',
    ori_price varchar(20) not null default '' comment '原价',
    cur_price varchar(20) not null default '' comment '现价',
    purchase_url varchar(1024) not null default '' comment '购买链接',
    purchase_type varchar(32) not null default 'normal' comment '商品购买类型：normal， 普通；yuyue, 预售商品；seckill，秒杀商品',
    status varchar(20) not null comment '状态:ready, yuyueing, purchasing, success, fail',
    is_delete tinyint unsigned not null default '0' comment '逻辑删除。0: 未删除;1:删除',
    create_time datetime not null,
    update_time datetime not null,
    index ix_userid_platform_sku(`user_id`, `platform`, `sku`)
)DEFAULT CHARSET=utf8mb4;
