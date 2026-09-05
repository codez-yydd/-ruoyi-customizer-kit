import request from '@/api/request'

/**
 * 上传结果（与 FileUpload / ImageUpload 现有 UploadResult 对齐）
 * Cloud POST /file/upload 的 body 为 { code, data: { name, url } }，
 * 此处把 data.url（缺省 data.name）映射到 url / fileName。
 */
export interface UploadResult {
  code: number
  msg: string
  /** 完整 URL 或相对路径（回显优先用 fileName） */
  url: string
  /** 与 url 同源：Cloud 无单体顶层 fileName，用 data.url 缺省 data.name */
  fileName: string
  newFileName: string
  originalFilename: string
}

/** 多文件上传结果（逗号拼接；Cloud 无 /common/uploads，由单文件循环拼出） */
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

/** Cloud 文件服务 R.data */
interface CloudFileData {
  name?: string
  url?: string
}

/** isRawResponse 后的完整 body */
interface CloudUploadBody {
  code?: number
  msg?: string
  data?: CloudFileData
}

function toUploadResult(body: CloudUploadBody): UploadResult {
  const data = body?.data
  const url = data?.url || data?.name || ''
  return {
    code: body?.code ?? 200,
    msg: body?.msg ?? '',
    url,
    fileName: url,
    newFileName: data?.name || '',
    originalFilename: data?.name || ''
  }
}

/** 单文件上传：POST /file/upload（Cloud 网关 Path=/file/**；multipart 字段名 file） */
export async function uploadFile(file: File): Promise<UploadResult> {
  const formData = new FormData()
  formData.append('file', file)
  // FormData 由 axios 自动生成 multipart 边界，不手动指定 Content-Type
  const body = await request.post<CloudUploadBody, CloudUploadBody>('/file/upload', formData, {
    isRawResponse: true
  })
  return toUploadResult(body)
}

/**
 * 多文件上传：Cloud 无 /common/uploads，循环调用 POST /file/upload。
 * 单文件场景（站点 Logo、ImageUpload、FileUpload）仍走 uploadFile。
 */
export async function uploadFiles(files: File[]): Promise<UploadsResult> {
  const results = await Promise.all(files.map((file) => uploadFile(file)))
  return {
    code: 200,
    msg: '',
    urls: results.map((item) => item.url).join(','),
    fileNames: results.map((item) => item.fileName).join(',')
  }
}

/** 头像上传：POST /system/user/profile/avatar（multipart 字段名 avatarfile） */
export function uploadAvatar(file: File): Promise<AvatarResult> {
  const formData = new FormData()
  formData.append('avatarfile', file)
  return request.post<AvatarResult, AvatarResult>('/system/user/profile/avatar', formData, {
    isRawResponse: true
  })
}
