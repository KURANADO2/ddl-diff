create database a_schema;

# init

use a_schema;

create table user (
id bigint primary key auto_increment comment '主键',
name varchar(30) null comment '姓名',
address varchar(50) null comment '地址',
number varchar(20) null comment '编号',
height float null comment '身高'
);

create index idx_name on user(name);

create index idx_multiple_field on user(name, address);

create database b_schema;

create table course (
id bigint primary key auto_increment comment '主键',
name varchar(30) null comment '课程名称',
teacher varchar(30) null comment '教师',
credit float null comment '学分'
);

# b schema same with a schema

create database a_schema;

use b_schema;

create table user (
id bigint primary key auto_increment comment '主键',
name varchar(30) null comment '姓名',
address varchar(50) null comment '地址',
number varchar(20) null comment '编号',
height float null comment '身高'
);

create index idx_name on user(name);

create index idx_multiple_field on user(name, address);

create table course (
id      bigint primary key auto_increment comment '主键',
name    varchar(30) null comment '课程名称',
teacher varchar(30) null comment '教师',
credit  float       null comment '学分'
);

# make the difference between a_schema and b_schema

use a_schema;
alter table user add column age int null comment '年龄';
alter table user add column create_time datetime not null default current_timestamp comment '创建时间';
alter table user modify column address varchar(100) not null comment '地址';
alter table user change column number phone varchar(20) null comment '电话号码';
alter table user drop column height;
alter table user add unique index uk_phone(phone);
alter table user drop index idx_name;
create table student(
id bigint primary key auto_increment comment '主键',
no varchar(30) null comment '学号',
name varchar(30) null comment '姓名'
);
create unique index uk_no on student(no);
drop index idx_multiple_field on user;
create index idx_multiple_filed on user(name, phone);
drop table course;

## Usage example

```bash
$ cargo build --release
$ target/release/ddl-diff -h                                                                                                                                                 48s  base  21:21:05 ↔ 127.0.0.1:7890
Usage: ddl-diff [OPTIONS] --original-user <ORIGINAL_USER> --original-password <ORIGINAL_PASSWORD> --original-host <ORIGINAL_HOST> --original-schema <ORIGINAL_SCHEMA> --target-user <TARGET_USER> --target-password <TARGET_PASSWORD> --target-host <TARGET_HOST> --target-schema <TARGET_SCHEMA>

Options:
      --original-user <ORIGINAL_USER>          
      --original-password <ORIGINAL_PASSWORD>  
      --original-host <ORIGINAL_HOST>          
      --original-port <ORIGINAL_PORT>          [default: 3306]
      --original-schema <ORIGINAL_SCHEMA>      
      --target-user <TARGET_USER>              
      --target-password <TARGET_PASSWORD>      
      --target-host <TARGET_HOST>              
      --target-port <TARGET_PORT>              [default: 3306]
      --target-schema <TARGET_SCHEMA>          
  -h, --help                                   Print help
  -V, --version                                Print version
$ target/release/ddl-diff --original-user root --original-password 123456 --original-host 127.0.0.1 --original-schema a_schema --target-user root --target-password 123456 --target-host 127.0.0.1 --target-schema b_schema

ALTER TABLE user ADD COLUMN phone varchar(20) NULL  COMMENT '电话号码';
ALTER TABLE user MODIFY COLUMN address varchar(100) NOT NULL  COMMENT '地址';
ALTER TABLE user ADD COLUMN age int NULL  COMMENT '年龄';
ALTER TABLE user ADD COLUMN create_time datetime NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间';
ALTER TABLE user DROP COLUMN number;
ALTER TABLE user DROP COLUMN height;
CREATE TABLE student(
name varchar(30) NULL  COMMENT '姓名',
no varchar(30) NULL  COMMENT '学号',
id bigint NOT NULL  COMMENT '主键'
);
DROP TABLE course;
CREATE UNIQUE INDEX PRIMARY ON student (id);
CREATE UNIQUE INDEX uk_no ON student (no);
CREATE UNIQUE INDEX uk_phone ON user (phone);
CREATE INDEX idx_multiple_filed ON user (name, phone);
DROP INDEX idx_name ON user;
DROP INDEX PRIMARY ON course;
DROP INDEX idx_multiple_field ON user;
```

