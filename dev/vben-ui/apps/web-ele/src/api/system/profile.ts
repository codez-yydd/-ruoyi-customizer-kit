import { baseRequestClient, requestClient } from '#/api/request';

/**
 * 个人中心相关接口（适配若依 SysProfileController，basePath: /system/user/profile）
 */

/** 用户实体（个人中心用到的字段） */
export interface ProfileUser {
  userId: number;
  userName: string;
  nickName: string;
  email?: string;
  phonenumber?: string;
  sex?: string;
  avatar?: string;
  dept?: { deptName: string };
  createTime?: string;
}

/** 个人中心详情响应 */
export interface ProfileResult {
  data: ProfileUser;
  roleGroup: string;
  postGroup: string;
}

/**
 * 获取个人中心详情（GET /system/user/profile）
 *
 * 该接口返回 {code, msg, data(user), roleGroup, postGroup}，roleGroup/postGroup 在顶层。
 * requestClient 的响应拦截器会取 data 字段（即 user 对象），从而丢掉 roleGroup/postGroup，
 * 因此这里改用 baseRequestClient（未挂解包拦截器）取完整响应再手动解析。
 */
export async function getProfileApi(): Promise<ProfileResult> {
  const resp = await baseRequestClient.get('/system/user/profile');
  const body = (resp as any)?.data ?? resp;
  return {
    data: body?.data ?? {},
    roleGroup: body?.roleGroup ?? '',
    postGroup: body?.postGroup ?? '',
  };
}

/** 修改个人资料（PUT /system/user/profile） */
export function updateProfileApi(data: {
  nickName: string;
  phonenumber?: string;
  email?: string;
  sex?: string;
}) {
  return requestClient.put('/system/user/profile', data);
}

/** 修改密码（PUT /system/user/profile/updatePwd） */
export function updateUserPwdApi(oldPassword: string, newPassword: string) {
  return requestClient.put('/system/user/profile/updatePwd', {
    oldPassword,
    newPassword,
  });
}

/**
 * 上传头像（POST /system/user/profile/avatar）
 * 若依字段名为 avatarfile，返回 {code, msg, imgUrl}（拦截器解包后取到 imgUrl）。
 */
export function uploadAvatarApi(file: File) {
  const formData = new FormData();
  formData.append('avatarfile', file);
  return requestClient.post<{ imgUrl: string }>(
    '/system/user/profile/avatar',
    formData,
    { headers: { 'Content-Type': 'multipart/form-data' } },
  );
}
