
# rust开发的某东购买商品的软件
  本软件由以下构成
  * 前端 *GUI*，支持OSX 64位、Windows 10 64位系统
  * 后台Api
      * 涉及技术：Mysql、redis
  
  通过前端代码调用相关api获取数据及保存数据

# 软件运行图样
![运行截图](https://media.giphy.com/media/pGuAMrlsdCC2K8sNRq/giphy.gif)
	
#### 运行本软件需要安装最新版rust
 
# 编译并执行后台软件
  * 进入文件夹 **jd_miaosha_rs_api**, 执行以下步骤：
	1. 编译： `cargo build --release`
	2. 拷贝文件夹 `jd_miaosha_rs_api/config` 到 `jd_miaosha_rs_api/target/release` 里,并修改里面相应的配置
	3. 运行软件: `cargo run --release`
	
# 编译并执行前端GUI软件
  * 进入文件夹 **jd_miaosha_rs**, 执行以下步骤：
	1. 编译： `cargo build --release`
	2. 拷贝文件夹 `jd_miaosha_rs/config` 到 `jd_miaosha_rs/target/release` 里,并修改`jd_miaosha_rs/target/release/config/config.toml`里的 **addr**， 并把它指向后台服务器地址
	3. 运行软件: `cargo run --release`
	

### 本软件只用作rust学习目的，任何商业行为与本品无关。
	


