import { requestClient } from '#/api/request';

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
 * 必须 rawResponse: true，否则拦截器只解包 data，丢掉 roleGroup/postGroup。
 * 不可用 baseRequestClient：其未挂 Authorization，会导致鉴权失败、页面字段全空。
 */
export async function getProfileApi(): Promise<ProfileResult> {
  const body = await requestClient.get<{
    code?: number;
    data?: ProfileUser;
    msg?: string;
    postGroup?: string;
    roleGroup?: string;
  }>('/system/user/profile', { rawResponse: true });

  return {
    data: body?.data ?? ({} as ProfileUser),
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
 * 若依字段名为 avatarfile，返回 {code, msg, imgUrl}（无 data 字段）。
 * 使用 rawResponse 保留 imgUrl，避免拦截器解包异常。
 */
export async function uploadAvatarApi(
  file: Blob | File,
  filename = 'avatar.png',
): Promise<{ imgUrl: string }> {
  const formData = new FormData();
  const name = file instanceof File ? file.name : filename;
  formData.append('avatarfile', file, name);
  const resp = await requestClient.post<{ imgUrl?: string; code?: number }>(
    '/system/user/profile/avatar',
    formData,
    {
      headers: { 'Content-Type': 'multipart/form-data' },
      rawResponse: true,
    },
  );
  return { imgUrl: resp?.imgUrl ?? '' };
}
