# 获取 APP 特征信息

可以快速获取 APP 的特征信息以供备案使用

目前只支持 iOS App 的信息提供

## 使用方法

1. 将打好即将发布到 app store 的 ipa 拖进软件中
2. 等待解析好之后就会显示特征信息
3. 点击保存按钮即可

## 本地运行项目

> > 前提条件：安装了 rust 和 tauri

```
yarn
yarn tauri dev
```

## 原理

- 解压 ipa 包后得到 xxx.app

- 执行以下命令可将 xxx.app 得到签名文件

```
codesign -d --extract-certificates xxxx.app
```

- 执行以下命令获取得 sha-1 和 modulus 信息

```
openssl x509 -fingerprint -sha1 -modulus -text -noout -in codesign0
```

- 解析 xxx.app 中的 Info.plist 文件得到 bundle id 和 名称以及图标路径

- 使用以下合集还原图片以保证在其他终端正常显示

```
xcrun -sdk iphoneos pngcrush \ -q -revert-iphone-optimizations -d AppIcon60x60@2x.png
```
