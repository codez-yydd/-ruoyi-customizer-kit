/**
 * 该文件可自行根据业务逻辑进行调整
 */
import type { HttpResponse } from '@vben/request';

import { useAppConfig } from '@vben/hooks';
import { preferences } from '@vben/preferences';
import {
  authenticateResponseInterceptor,
  errorMessageResponseInterceptor,
  RequestClient,
} from '@vben/request';
import { useAccessStore } from '@vben/stores';

import { ElMessage } from 'element-plus';

import { useAuthStore } from '#/store';

import { refreshTokenApi } from './core';

const { apiURL } = useAppConfig(import.meta.env, import.meta.env.PROD);

function createRequestClient(baseURL: string) {
  const client = new RequestClient({
    baseURL,
  });

  /**
   * 重新认证逻辑
   */
  async function doReAuthenticate() {
    console.warn('Access token or refresh token is invalid or expired. ');
    const accessStore = useAccessStore();
    const authStore = useAuthStore();
    accessStore.setAccessToken(null);
    if (
      preferences.app.loginExpiredMode === 'modal' &&
      accessStore.isAccessChecked
    ) {
      accessStore.setLoginExpired(true);
    } else {
      await authStore.logout();
    }
  }

  /**
   * 刷新token逻辑
   */
  async function doRefreshToken() {
    const accessStore = useAccessStore();
    const resp = await refreshTokenApi();
    const newToken = resp.data;
    accessStore.setAccessToken(newToken);
    return newToken;
  }

  function formatToken(token: null | string) {
    return token ? `Bearer ${token}` : null;
  }

  // 请求头处理
  client.addRequestInterceptor({
    fulfilled: async (config) => {
      const accessStore = useAccessStore();

      config.headers.Authorization = formatToken(accessStore.accessToken);
      config.headers['Accept-Language'] = preferences.app.locale;
      return config;
    },
  });

  // response数据解构（适配若依 AjaxResult 契约）
  // 若依有三种成功响应形态，拦截器需统一处理：
  //   ① 普通对象：{code:200, msg, data}        → 返回 data
  //   ② 分页列表：{code:200, msg, rows, total} → 返回 {rows, total}（业务层自行取用）
  //   ③ 扁平聚合：{code:200, msg, data, roles, posts, ...} → 需保留全部顶层字段，调用方
  //      在请求 config 中设置 rawResponse: true 即可跳过自动解包，拿到完整响应体
  //   ④ 登录特例：{code:200, msg, token}        → 登录用 baseRequestClient 单独处理，不走此处
  //
  // 错误响应关键点：若依认证失败等错误统一以 HTTP 200 + 业务码返回（如
  //   {code:401, msg:"请求访问：/getInfo，认证失败..."}），HTTP 层是成功的，
  //   axios 不会 reject。若直接 throw 原始 response，后续认证拦截器只会看到
  //   status=200，既不会跳登录页、也不会提示 msg，最终页面无限加载。
  //   因此这里把若依业务码映射为标准错误对象（response.status = 业务码），
  //   让后续 authenticateResponseInterceptor / errorMessageResponseInterceptor
  //   能按 HTTP 状态码语义统一处理：code 401 → 触发重新认证并跳登录页，其它 → 提示 msg。
  client.addResponseInterceptor<HttpResponse>({
    fulfilled: (response) => {
      const { data: responseData, status, config } = response;

      const { code } = responseData ?? {};

      if (status >= 200 && status < 400 && code === 200) {
        // 请求时设置 rawResponse: true 的接口（如用户详情，需保留 roles/posts 等顶层字段），
        // 直接原样返回完整响应体，不做 data 解包
        if ((config as any)?.rawResponse) {
          return responseData;
        }
        // 优先取 data；若依分页接口无 data 而有 rows，则原样返回含 rows/total 的对象
        if (responseData.data !== undefined) {
          return responseData.data;
        }
        return responseData;
      }

      // 若依风格：HTTP 成功但业务失败（code !== 200）。
      // 构造标准化错误：把若依业务码放进 response.status，原始响应体放进 response.data，
      // 这样认证拦截器（识别 401）与错误提示（读取 msg）都能正确工作。
      const bizError = Object.assign(new Error(responseData?.msg), {
        config,
        response: {
          ...response,
          status: code ?? status,
          data: responseData,
        },
        isAxiosError: false,
      });
      throw bizError;
    },
  });

  // token过期的处理
  client.addResponseInterceptor(
    authenticateResponseInterceptor({
      client,
      doReAuthenticate,
      doRefreshToken,
      enableRefreshToken: preferences.app.enableRefreshToken,
      formatToken,
    }),
  );

  // 通用的错误处理,如果没有进入上面的错误处理逻辑，就会进入这里
  client.addResponseInterceptor(
    errorMessageResponseInterceptor((msg: string, error) => {
      // 这里可以根据业务进行定制,你可以拿到 error 内的信息进行定制化处理，根据不同的 code 做不同的提示，而不是直接使用 message.error 提示 msg
      // 适配若依：错误体统一为 {code, msg}，优先用后端返回的 msg 提示
      const responseData = error?.response?.data ?? {};
      const errorMessage =
        responseData?.msg ?? responseData?.message ?? responseData?.error ?? '';
      // 如果没有错误信息，则会根据状态码进行提示
      ElMessage.error(errorMessage || msg);
    }),
  );

  return client;
}

export const requestClient = createRequestClient(apiURL);

export const baseRequestClient = new RequestClient({ baseURL: apiURL });
