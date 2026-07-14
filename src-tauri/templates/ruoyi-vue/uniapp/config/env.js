const ENV = {
  development: {
    baseUrl: '{{API_BASE_URL_DEV}}'
  },
  production: {
    baseUrl: '{{API_BASE_URL_PROD}}'
  }
}

// 小程序环境判断
const currentEnv = process.env.NODE_ENV || 'development'

export default ENV[currentEnv] || ENV.development
