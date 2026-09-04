import request from '@/api/request'

/**
 * POST /common/upload 响应体（字段在顶层）
 * url 为后端拼好的完整地址（域名与前端 dev 环境不一致，回显优先用 fileName）
 */
export interface UploadResult {
  code: number
  msg: string
  /** 完整 URL（仅作参考，回显请用 fileName 拼 VITE_APP_BASE_API） */
  url: string
  /** 相对路径（/profile/...），回显时拼 VITE_APP_BASE_API 前缀 */
  fileName: string
  newFileName: string
  originalFilename: string
}

/** POST /common/uploads 响应体（多文件，逗号拼接） */
export interface UploadsResult {
  code: number
  msg: string
  urls: string
  fileNames: string
}

/** POST /system/user/profile/avatar 响应体（字段在顶层） */
export interface AvatarResult {
  code: number
  msg: string
  imgUrl: string
}

/** 单文件上传：POST /common/upload（multipart 字段名 file） */
export function uploadFile(file: File): Promise<UploadResult> {
  const formData = new FormData()
  formData.append('file', file)
  // FormData 由 axios 自动生成 multipart 边界，不手动指定 Content-Type
  return request.post<UploadResult, UploadResult>('/common/upload', formData, {
    isRawResponse: true
  })
}

/** 多文件上传：POST /common/uploads（multipart 字段名 files，结果为逗号拼接串） */
export function uploadFiles(files: File[]): Promise<UploadsResult> {
  const formData = new FormData()
  files.forEach((file) => formData.append('files', file))
  return request.post<UploadsResult, UploadsResult>('/common/uploads', formData, {
    isRawResponse: true
  })
}

/** 头像上传：POST /system/user/profile/avatar（multipart 字段名 avatarfile） */
export function uploadAvatar(file: File): Promise<AvatarResult> {
  const formData = new FormData()
  formData.append('avatarfile', file)
  return request.post<AvatarResult, AvatarResult>('/system/user/profile/avatar', formData, {
    isRawResponse: true
  })
}
