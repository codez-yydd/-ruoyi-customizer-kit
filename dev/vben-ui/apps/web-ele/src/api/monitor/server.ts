import { requestClient } from '#/api/request';

/**
 * 服务器监控（移植自 ruoyi-ui/src/api/monitor/server.js）
 */
export interface CpuInfo {
  cpuNum: number;
  total: number;
  sys: number;
  used: number;
  wait: number;
  free: number;
  user?: number;
}

export interface MemInfo {
  total: number;
  used: number;
  free: number;
  usage: number;
}

export interface JvmInfo {
  total: number;
  used: number;
  free: number;
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

// 获取服务器信息
export function getServer() {
  return requestClient.get<{ data: ServerInfo }>('/monitor/server');
}
