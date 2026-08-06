/**
 * vue-cropper 类型声明。
 * 官方 package typings 引用了未带声明的 .vue 源码，会导致 vue-tsc 报错，
 * 因此业务侧统一从 dist 产物导入，并用本声明补齐类型。
 */
declare module 'vue-cropper/dist/vue-cropper.es.js' {
  import type { DefineComponent } from 'vue';

  export interface VueCropperInstance {
    changeScale: (num: number) => void;
    rotateLeft: () => void;
    rotateRight: () => void;
    getCropBlob: (callback: (blob: Blob) => void) => void;
    getCropData: (callback: (data: string) => void) => void;
    refresh: () => void;
  }

  export const VueCropper: DefineComponent<Record<string, any>, any, any>;

  const globalCropper: {
    version: string;
    install: (app: unknown) => void;
    VueCropper: typeof VueCropper;
  };

  export default globalCropper;
}

declare module 'vue-cropper/dist/index.css';
