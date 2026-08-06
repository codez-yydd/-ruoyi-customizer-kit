import { requestClient } from '#/api/request';

/**
 * 服务器监控（移植自 ruoyi-ui/src/api/monitor/server.js）
 */
export interface CpuInfo {
  /** 核心数 */
  cpuNum: number;
  /** CPU 总使用率（百分比） */
  total: number;
  /** 系统使用率（百分比） */
  sys: number;
  /** 用户使用率（百分比） */
  used: number;
  /** 等待率（百分比） */
  wait: number;
  /** 空闲率（百分比） */
  free: number;
}

export interface MemInfo {
  /** 总内存（G） */
  total: number;
  /** 已用内存（G） */
  used: number;
  /** 剩余内存（G） */
  free: number;
  /** 使用率（百分比） */
  usage: number;
}

export interface JvmInfo {
  /** 当前占用内存（M） */
  total: number;
  /** 最大可用内存（M） */
  max: number;
  /** 已用内存（M） */
  used: number;
  /** 剩余内存（M） */
  free: number;
  /** 使用率（百分比） */
  usage: number;
  name: string;
  version: string;
  home: string;
  startTime: string;
  runTime: string;
  inputArgs: string;
}

export interface SysInfo {
  computerName: string;
  computerIp: string;
  osName: string;
  osArch: string;
  userDir: string;
}

export interface SysFileInfo {
  dirName: string;
  sysTypeName: string;
  typeName: string;
  total: string;
  free: string;
  used: string;
  usage: number;
}

export interface ServerInfo {
  cpu: CpuInfo;
  mem: MemInfo;
  jvm: JvmInfo;
  sys: SysInfo;
  sysFiles: SysFileInfo[];
}

/**
 * 获取服务器监控信息。
 * requestClient 已解包 AjaxResult.data，直接返回 Server 本体。
 */
export function getServer() {
  return requestClient.get<ServerInfo>('/monitor/server');
}
