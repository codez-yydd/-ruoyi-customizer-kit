# {{PROJECT_NAME}} 小程序

基于 UniApp 开发的微信小程序项目。

## 项目信息

- 项目名称：{{PROJECT_NAME}}
- 小程序目录：{{UNIAPP_NAME}}
- 模块前缀：{{MODULE_PREFIX}}

## 快速开始

### 安装依赖

```bash
npm install
```

### 开发模式（微信小程序）

```bash
npm run dev:mp-weixin
```

编译后使用微信开发者工具导入 `dist/dev/mp-weixin` 目录。

### 生产构建

```bash
npm run build:mp-weixin
```

编译后使用微信开发者工具导入 `dist/build/mp-weixin` 目录。

## 配置说明

### manifest.json

在 `manifest.json` 中填写微信小程序 AppID：

```json
{
  "mp-weixin": {
    "appid": "你的微信小程序 AppID"
  }
}
```

### API 地址

在 `config/env.js` 中配置开发/生产环境的后端接口地址：

```javascript
const ENV = {
  development: {
    baseUrl: 'http://localhost:8080'  // 开发环境
  },
  production: {
    baseUrl: 'https://api.example.com'  // 生产环境
  }
}
```

### 后端微信配置

后端 `application-dev.yaml` 和 `application-prod.yaml` 中已预留微信小程序配置：

```yaml
{{MODULE_PREFIX}}:
  wx:
    appid: ''        # 微信小程序 AppID
    appsecret: ''    # 微信小程序 AppSecret
  wechat:
    pay:
      enabled: false
      mock: true
      mch-id: ''
      # ... 其他支付配置
```

请根据实际信息填写以上配置。

## 登录说明

小程序登录流程：

1. 调用 `uni.login` 获取微信 code
2. 将 code 发送到后端 `/app/{{MODULE_PREFIX}}/auth/wechat-login`
3. 后端返回 token，保存到本地存储
4. 后续请求自动携带 token

> 注意：后端需要实现对应的小程序登录接口，本模板仅提供前端调用框架。

## 项目结构

```
{{UNIAPP_NAME}}/
├── pages.json          # 页面配置
├── manifest.json       # 应用配置
├── package.json        # 依赖管理
├── App.vue             # 应用入口
├── main.js             # 入口文件
├── uni.scss            # 全局样式变量
├── pages/              # 页面目录
│   ├── index/          # 首页
│   ├── mine/           # 我的
│   └── auth/           # 登录
├── api/                # 接口封装
│   ├── request.js      # 请求工具
│   ├── auth.js         # 登录接口
│   └── user.js         # 用户接口
├── config/             # 配置
│   ├── env.js          # 环境配置
│   └── app.js          # 应用配置
├── utils/              # 工具
│   ├── auth.js         # 登录状态
│   └── storage.js      # 本地存储
└── static/             # 静态资源
```

## 常见问题

### Q: 如何修改小程序 AppID？
A: 编辑 `manifest.json` 中的 `mp-weixin.appid` 字段。

### Q: 如何修改后端接口地址？
A: 编辑 `config/env.js` 中的 `baseUrl`。

### Q: 登录不生效？
A: 请确认后端已实现 `/app/{{MODULE_PREFIX}}/auth/wechat-login` 接口。

## 版权

{{COPYRIGHT}}
