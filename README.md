A tool to compare two databases and generate a diff for MySQL.

## Install

### Cargo

```bash
$ cargo install ddl-diff
```

### From Source

```bash
$ git clone git@github.com:KURANADO2/ddl-diff.git
$ cd ddl-diff
$ cargo build --release
```

Then you can use the binary in `target/release/ddl-diff` or add it to your PATH.

## Options

```bash
$ ddl-diff -h                                                                                                                                                                                                                 base  20:56:22
A tool to compare two databases and generate a diff for MySQL.

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
```

## Usage

```bash
$ ddl-diff --original-user root --original-password 123456 --original-host 127.0.0.1 --original-schema a_schema --target-user root --target-password 123456 --target-host 127.0.0.1 --target-schema b_schema
```

Then it will be output some content about the difference between two database.

## Example

### create database a_schema

```mysql
drop database a_schema;
create database a_schema;

use a_schema;

create table user
(
    id      bigint primary key auto_increment comment '主键',
    name    varchar(30) null comment '姓名',
    address varchar(50) null comment '地址',
    number  varchar(20) null comment '编号',
    height  float       null comment '身高'
);

create index idx_name on user (name);

create index idx_multiple_field on user (name, address);

create table course
(
    id      bigint primary key auto_increment comment '主键',
    name    varchar(30) null comment '课程名称',
    teacher varchar(30) null comment '教师',
    credit  float       null comment '学分'
);
```

### create b_schema

The b_schema is the same with a_schema.

```mysql
drop database b_schema;
create database b_schema;

use b_schema;

create table user
(
    id      bigint primary key auto_increment comment '主键',
    name    varchar(30) null comment '姓名',
    address varchar(50) null comment '地址',
    number  varchar(20) null comment '编号',
    height  float       null comment '身高'
);

create index idx_name on user (name);

create index idx_multiple_field on user (name, address);

create table course
(
    id      bigint primary key auto_increment comment '主键',
    name    varchar(30) null comment '课程名称',
    teacher varchar(30) null comment '教师',
    credit  float       null comment '学分'
);
```

### make the difference between a_schema and b_schema

```mysql
use a_schema;
alter table user add column age int null comment '年龄';
alter table user add column create_time datetime not null default current_timestamp comment '创建时间';
alter table user modify column address varchar(100) not null comment '地址';
alter table user change column number phone varchar(20) null comment '电话号码';
alter table user drop column height;
alter table user add unique index uk_phone (phone);
alter table user drop index idx_name;
create table student
(
    id   bigint primary key auto_increment comment '主键',
    no   varchar(30) null comment '学号',
    name varchar(30) null comment '姓名'
);
create unique index uk_no on student (no);
drop index idx_multiple_field on user;
create index idx_multiple_filed on user (name, phone);
drop table course;
```

### Run the program

```bash
$ ddl-diff --original-user root --original-password 123456 --original-host 127.0.0.1 --original-schema a_schema --target-user root --target-password 123456 --target-host 127.0.0.1 --target-schema b_schema
```

Output the following:

```mysql
use b_schema;
CREATE TABLE student(
name varchar(30) NULL  COMMENT '姓名',
id bigint NOT NULL  COMMENT '主键',
no varchar(30) NULL  COMMENT '学号'
);
ALTER TABLE user ADD COLUMN create_time datetime NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间';
ALTER TABLE user ADD COLUMN age int NULL  COMMENT '年龄';
ALTER TABLE user MODIFY COLUMN address varchar(100) NOT NULL  COMMENT '地址';
ALTER TABLE user ADD COLUMN phone varchar(20) NULL  COMMENT '电话号码';
ALTER TABLE user DROP COLUMN height;
ALTER TABLE user DROP COLUMN number;
DROP TABLE course;
CREATE UNIQUE INDEX uk_phone ON user (phone);
CREATE UNIQUE INDEX uk_no ON student (no);
CREATE INDEX idx_multiple_filed ON user (name, phone);
ALTER TABLE student ADD PRIMARY KEY (id);
DROP INDEX idx_multiple_field ON user;
DROP INDEX idx_name ON user;
```

## Note

If you take a closer look at the output above, you’ll notice that the program fails to correctly identify cases where
field names have been modified. This is because MySQL’s information_schema does not store a unique ID for fields, making
it impossible to distinguish between a field being renamed and a field being deleted and a new one added. The same issue
applies to table name changes.

One possible solution is to calculate the edit distance of field properties, but this method is not always accurate.
If you use table structure comparison features in tools like Navicat or DataGrip, you’ll find that they also cannot
handle such situations.

Therefore, when using this program, be sure to manually verify the output for correctness before executing the SQL
statements.