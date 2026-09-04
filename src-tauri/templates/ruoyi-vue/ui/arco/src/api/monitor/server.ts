import request from '@/api/request'

/** CPU 信息 */
export interface ServerCpu {
  /** 核心数量 */
  cpuNum: number
  /** CPU 总使用率（%） */
  total: number
  /** CPU 系统使用率（%） */
  sys: number
  /** CPU 用户使用率（%） */
  used: number
  /** CPU 当前等待率（%） */
  wait: number
  /** CPU 当前空闲率（%） */
  free: number
}

/** 内存信息（单位 GB） */
export interface ServerMem {
  total: number
  used: number
  free: number
  /** 使用率（%） */
  usage: number
}

/** JVM 信息（内存单位 MB） */
export interface ServerJvm {
  total: number
  max: number
  free: number
  version: string
  home: string
  startTime: string
  runTime: string
  /** 使用率（%） */
  usage: number
  used: number
  /** 启动参数 */
  inputArgs: string
  name?: string
}

/** 服务器系统信息 */
export interface ServerSys {
  computerName: string
  computerIp: string
  osName: string
  osArch: string
  /** 项目路径 */
  userDir: string
}

/** 磁盘挂载信息 */
export interface ServerFile {
  /** 盘符路径 */
  dirName: string
  /** 文件系统 */
  sysTypeName: string
  /** 盘符类型 */
  typeName: string
  total: string
  free: string
  used: string
  /** 已用百分比 */
  usage: number
}

/** GET /monitor/server 响应 data（以实测字段为准） */
export interface ServerInfo {
  cpu: ServerCpu
  mem: ServerMem
  jvm: ServerJvm
  sys: ServerSys
  sysFiles: ServerFile[]
}

/** 服务监控信息：GET /monitor/server */
export function getServerInfo(): Promise<ServerInfo> {
  return request.get<ServerInfo, ServerInfo>('/monitor/server')
}
