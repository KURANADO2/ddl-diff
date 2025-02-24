A tool to compare two databases and generate a diff for MariaDB.

## Install

### Use Cargo

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
A tool to compare two databases and generate a diff for MariaDB.

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
$ ddl-diff \
--original-user root \
--original-password 123456 \
--original-host 127.0.0.1 \
--original-port 3306 \
--original-schema a_schema \
--target-user root \
--target-password 123456 \
--target-host 127.0.0.1 \
--target-port 3306 \
--target-schema b_schema
```

Then it will be output some content about the difference between two database.

## Example

### create a_schema

```mariadb
drop database a_schema;
create database a_schema;

use a_schema;

create table user (
                      id bigint comment '主键',
                      name varchar(30) null comment '姓名',
                      address varchar(50) null comment '地址',
                      number varchar(20) null comment '编号',
                      height float null comment '身高'
);

create table teacher (
                         name varchar(30) null comment '姓名'
);

create index idx_name on user(name);

create index idx_multiple_field on user(name, address);

create table course (
                        id      bigint primary key auto_increment comment '主键',
                        name    varchar(30) null comment '课程名称',
                        teacher varchar(30) null comment '教师',
                        credit  float       null comment '学分'
);

create table pig (
                     id bigint not null comment '名称',
                     weight bigint not null comment '重量'
);
```

### create b_schema

The b_schema is the same with a_schema.

```mariadb
drop database b_schema;
create database b_schema;

use b_schema;

create table user (
                      id bigint comment '主键',
                      name varchar(30) null comment '姓名',
                      address varchar(50) null comment '地址',
                      number varchar(20) null comment '编号',
                      height float null comment '身高'
);

create table teacher (
                         name varchar(30) null comment '姓名'
);

create index idx_name on user(name);

create index idx_multiple_field on user(name, address);

create table course (
                        id      bigint primary key auto_increment comment '主键',
                        name    varchar(30) null comment '课程名称',
                        teacher varchar(30) null comment '教师',
                        credit  float       null comment '学分'
);

create table pig (
                     id bigint not null comment '名称',
                     weight bigint not null comment '重量'
);
```

### make the difference between a_schema and b_schema

```mariadb
use a_schema;
alter table user modify column id bigint primary key auto_increment not null comment '主键';
alter table user add column age int default 18 null comment '年龄';
alter table user add column create_time datetime not null default current_timestamp comment '创建时间';
alter table user modify column address varchar(100) not null default 'Shanghai' comment '地址';
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
create index idx_multiple_field on user(name, phone);
drop table course;
alter table user add unique index uk_test(age, create_time);
alter table student add index idx_name(name);
alter table teacher add COLUMN `id` bigint NOT NULL primary key AUTO_INCREMENT COMMENT '主键' FIRST;
alter table pig add unique index uk_id_weight(id, weight);
alter table pig modify column id bigint not null primary key auto_increment comment '名称';
```

### Run the program

```bash
$ ddl-diff \
--original-user root \
--original-password 123456 \
--original-host 127.0.0.1 \
--original-schema a_schema \
--target-user root \
--target-password 123456 \
--target-host 127.0.0.1 \
--target-schema b_schema
```

Output the following:

```mariadb
use b_schema;
CREATE TABLE `student`(
                          `id` bigint(20) NOT NULL  PRIMARY KEY AUTO_INCREMENT COMMENT '主键',
                          `no` varchar(30) NULL DEFAULT NULL  COMMENT '学号',
                          `name` varchar(30) NULL DEFAULT NULL  COMMENT '姓名',
                          INDEX `idx_name` (`name`) USING BTREE,
                          UNIQUE INDEX `uk_no` (`no`) USING BTREE);
ALTER TABLE `user` DROP INDEX `idx_name`;
ALTER TABLE `user` DROP INDEX `idx_multiple_field`;
ALTER TABLE `user` ADD COLUMN `phone` varchar(20) NULL DEFAULT NULL  COMMENT '电话号码' AFTER `address`;
ALTER TABLE `user` ADD COLUMN `age` int(11) NULL DEFAULT 18  COMMENT '年龄' AFTER `phone`;
ALTER TABLE `user` ADD COLUMN `create_time` datetime NOT NULL DEFAULT current_timestamp()  COMMENT '创建时间' AFTER `age`;
ALTER TABLE `user` MODIFY COLUMN `id` bigint(20) NOT NULL  PRIMARY KEY AUTO_INCREMENT COMMENT '主键' FIRST;
ALTER TABLE `user` MODIFY COLUMN `address` varchar(100) NOT NULL DEFAULT 'Shanghai'  COMMENT '地址' AFTER `name`;
ALTER TABLE `user` DROP COLUMN `height`;
ALTER TABLE `user` DROP COLUMN `number`;
ALTER TABLE `user` ADD INDEX `idx_multiple_field` (`name`,`phone`) USING BTREE;
ALTER TABLE `user` ADD UNIQUE INDEX `uk_test` (`age`,`create_time`) USING BTREE;

ALTER TABLE `user` ADD UNIQUE INDEX `uk_phone` (`phone`) USING BTREE;
ALTER TABLE `pig` MODIFY COLUMN `id` bigint(20) NOT NULL  PRIMARY KEY AUTO_INCREMENT COMMENT '名称' FIRST;

ALTER TABLE `pig` ADD UNIQUE INDEX `uk_id_weight` (`id`,`weight`) USING BTREE;
ALTER TABLE `teacher` ADD COLUMN `id` bigint(20) NOT NULL  PRIMARY KEY AUTO_INCREMENT COMMENT '主键' FIRST;

DROP TABLE IF EXISTS `course`;
```

## Note

If you take a closer look at the output above, you’ll notice that the program fails to correctly identify cases where
field names have been modified. This is because MariaDB’s information_schema does not store a unique ID for fields, making
it impossible to distinguish between a field being renamed and a field being deleted and a new one added. The same issue
applies to table name changes.

One possible solution is to calculate the edit distance of field properties, but this method is not always accurate.
If you use table structure comparison features in tools like Navicat or DataGrip, you’ll find that they also cannot
handle such situations.

Therefore, when using this program, be sure to manually verify the output for correctness before executing the SQL
statements.