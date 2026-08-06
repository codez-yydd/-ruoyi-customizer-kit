import { requestClient } from '#/api/request';

/** 通知公告 */
export interface SysNotice {
  noticeId: number;
  noticeTitle: string;
  noticeType: string;
  noticeContent?: string;
  status: string;
  createBy?: string;
  createTime?: string;
  /** 当前用户是否已读（顶部公告接口返回） */
  isRead?: boolean;
}

/** 公告已读用户 */
export interface NoticeReadUser {
  userName: string;
  nickName: string;
  deptName?: string;
  phonenumber?: string;
  readTime?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

/** 查询公告列表 */
export function listNotice(query: Record<string, any>) {
  return requestClient.get<TableResult<SysNotice>>('/system/notice/list', { params: query });
}

/**
 * 查询公告详情
 * 响应拦截器自动解包 data，返回值即公告对象本身（参考 getPost / getRole）
 */
export function getNotice(noticeId: number) {
  return requestClient.get<SysNotice>(`/system/notice/${noticeId}`);
}

/** 新增公告 */
export function addNotice(data: Partial<SysNotice>) {
  return requestClient.post('/system/notice', data);
}

/** 修改公告 */
export function updateNotice(data: Partial<SysNotice>) {
  return requestClient.put('/system/notice', data);
}

/** 删除公告（支持单个 ID 或批量 ID 数组） */
export function delNotice(noticeId: number | number[]) {
  return requestClient.delete(`/system/notice/${noticeId}`);
}

/**
 * 首页顶部公告列表（带已读状态）
 *
 * 后端返回扁平结构 {code, msg, data: SysNotice[], unreadCount}，
 * 必须 rawResponse 保留 unreadCount，否则会被全局解包丢掉。
 */
export function listNoticeTop() {
  return requestClient.get<{ data: SysNotice[]; unreadCount: number }>(
    '/system/notice/listTop',
    { rawResponse: true },
  );
}

/** 标记单条公告已读 */
export function markNoticeRead(noticeId: number) {
  return requestClient.post('/system/notice/markRead', null, {
    params: { noticeId },
  });
}

/** 批量标记已读（ids 为逗号分隔的公告 ID） */
export function markNoticeReadAll(ids: string) {
  return requestClient.post('/system/notice/markReadAll', null, {
    params: { ids },
  });
}

/** 查询公告已读用户列表 */
export function listNoticeReadUsers(query: Record<string, any>) {
  return requestClient.get<TableResult<NoticeReadUser>>(
    '/system/notice/readUsers/list',
    { params: query },
  );
}
