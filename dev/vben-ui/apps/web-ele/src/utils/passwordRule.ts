import { computed, ref } from 'vue';

/**
 * 密码强度规则（适配若依 sys.account.chrtype）
 *
 * chrtype 说明：
 *   0 - 任意字符（默认，禁止非法字符）
 *   1 - 纯数字（0-9）
 *   2 - 纯字母（a-z / A-Z）
 *   3 - 字母 + 数字（必须同时包含）
 *   4 - 字母 + 数字 + 特殊字符（必须同时包含）
 */

const SESSION_KEY = 'pwrChrtype';

/** 各类型对应的正则与错误提示 */
const PWD_RULES: Record<string, { pattern: RegExp; message: string }> = {
  '0': {
    pattern: /^[^<>"'|\\]+$/,
    message: '密码不能包含非法字符：< > " \' \\ |',
  },
  '1': { pattern: /^[0-9]+$/, message: '密码只能为数字（0-9）' },
  '2': {
    pattern: /^[a-zA-Z]+$/,
    message: '密码只能为英文字母（a-z、A-Z）',
  },
  '3': {
    pattern: /^(?=.*[a-zA-Z])(?=.*[0-9])[a-zA-Z0-9]+$/,
    message: '密码必须同时包含字母和数字',
  },
  '4': {
    pattern:
      /^(?=.*[A-Za-z])(?=.*\d)(?=.*[~!@#$%^&*()\-=_+])[A-Za-z\d~!@#$%^&*()\-=_+]+$/,
    message:
      '密码必须同时包含字母、数字和特殊字符（~!@#$%^&*()-=_+）',
  },
};

function readSessionChrType(): string {
  try {
    return sessionStorage.getItem(SESSION_KEY) || '0';
  } catch {
    return '0';
  }
}

/** 当前密码字符范围类型（登录 getInfo 后会同步） */
const pwdChrType = ref(readSessionChrType());

/**
 * 同步后端下发的密码字符范围配置。
 * 应在 /getInfo 成功后调用，供个人中心改密、用户管理重置密码等校验使用。
 */
export function setPwdChrType(chrType?: string | null) {
  const next = chrType || '0';
  pwdChrType.value = next;
  try {
    sessionStorage.setItem(SESSION_KEY, next);
  } catch {
    // sessionStorage 不可用时仅保留内存值
  }
}

export function usePasswordRule() {
  /** 通用密码校验（新增用户等） */
  const pwdValidator = computed(() => {
    const rule = PWD_RULES[pwdChrType.value] ?? PWD_RULES['0']!;
    return [
      { required: true, message: '密码不能为空', trigger: 'blur' },
      {
        min: 6,
        max: 20,
        message: '密码长度必须介于 6 和 20 之间',
        trigger: 'blur',
      },
      { pattern: rule.pattern, message: rule.message, trigger: 'blur' },
    ];
  });

  /** 个人中心修改密码校验 */
  const infoPwdValidator = computed(() => {
    const rule = PWD_RULES[pwdChrType.value] ?? PWD_RULES['0']!;
    return [
      { required: true, message: '新密码不能为空', trigger: 'blur' },
      {
        min: 6,
        max: 20,
        message: '新密码长度必须介于 6 和 20 之间',
        trigger: 'blur',
      },
      { pattern: rule.pattern, message: rule.message, trigger: 'blur' },
    ];
  });

  return {
    pwdChrType,
    pwdValidator,
    infoPwdValidator,
  };
}
