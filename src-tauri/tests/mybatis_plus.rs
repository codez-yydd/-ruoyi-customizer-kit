// 集成测试：验证 MyBatis-Plus 集成（依赖、配置类、generator 模板适配）

use ruoyi_forge_lib::core::mybatis_plus;
use ruoyi_forge_lib::core::snowflake;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

// ---------- 官方生成器模板夹具（复制自 RuoYi 官方 ruoyi-generator vm/java，自包含） ----------

const OFFICIAL_MAPPER_VM: &str = r#"package ${packageName}.mapper;

import java.util.List;
import ${packageName}.domain.${ClassName};
#if($table.sub)
import ${packageName}.domain.${subClassName};
#end

/**
 * ${functionName}Mapper接口
 *
 * @author ${author}
 * @date ${datetime}
 */
public interface ${ClassName}Mapper
{
    /**
     * 查询${functionName}
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return ${functionName}
     */
    public ${ClassName} select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});

    /**
     * 查询${functionName}列表
     *
     * @param ${className} ${functionName}
     * @return ${functionName}集合
     */
    public List<${ClassName}> select${ClassName}List(${ClassName} ${className});

    /**
     * 新增${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
    public int insert${ClassName}(${ClassName} ${className});

    /**
     * 修改${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
    public int update${ClassName}(${ClassName} ${className});

    /**
     * 删除${functionName}
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return 结果
     */
    public int delete${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});

    /**
     * 批量删除${functionName}
     *
     * @param ${pkColumn.javaField}s 需要删除的数据主键集合
     * @return 结果
     */
    public int delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaType}[] ${pkColumn.javaField}s);
#if($table.sub)

    /**
     * 批量删除${subTable.functionName}
     *
     * @param ${pkColumn.javaField}s 需要删除的数据主键集合
     * @return 结果
     */
    public int delete${subClassName}By${subTableFkClassName}s(${pkColumn.javaType}[] ${pkColumn.javaField}s);

    /**
     * 批量新增${subTable.functionName}
     *
     * @param ${subclassName}List ${subTable.functionName}列表
     * @return 结果
     */
    public int batch${subClassName}(List<${subClassName}> ${subclassName}List);


    /**
     * 通过${functionName}主键删除${subTable.functionName}信息
     *
     * @param ${pkColumn.javaField} ${functionName}ID
     * @return 结果
     */
    public int delete${subClassName}By${subTableFkClassName}(${pkColumn.javaType} ${pkColumn.javaField});
#end
}
"#;

const OFFICIAL_SERVICE_VM: &str = r#"package ${packageName}.service;

import java.util.List;
import ${packageName}.domain.${ClassName};

/**
 * ${functionName}Service接口
 *
 * @author ${author}
 * @date ${datetime}
 */
public interface I${ClassName}Service
{
    /**
     * 查询${functionName}
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return ${functionName}
     */
    public ${ClassName} select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});

    /**
     * 查询${functionName}列表
     *
     * @param ${className} ${functionName}
     * @return ${functionName}集合
     */
    public List<${ClassName}> select${ClassName}List(${ClassName} ${className});

    /**
     * 新增${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
    public int insert${ClassName}(${ClassName} ${className});

    /**
     * 修改${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
    public int update${ClassName}(${ClassName} ${className});

    /**
     * 批量删除${functionName}
     *
     * @param ${pkColumn.javaField}s 需要删除的${functionName}主键集合
     * @return 结果
     */
    public int delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaType}[] ${pkColumn.javaField}s);

    /**
     * 删除${functionName}信息
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return 结果
     */
    public int delete${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});
}
"#;

const OFFICIAL_SERVICE_IMPL_VM: &str = r#"package ${packageName}.service.impl;

import java.util.List;
#foreach ($column in $columns)
#if($column.javaField == 'createTime' || $column.javaField == 'updateTime')
import com.ruoyi.common.utils.DateUtils;
#break
#end
#end
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
#if($table.sub)
import java.util.ArrayList;
import com.ruoyi.common.utils.StringUtils;
import org.springframework.transaction.annotation.Transactional;
import ${packageName}.domain.${subClassName};
#end
import ${packageName}.mapper.${ClassName}Mapper;
import ${packageName}.domain.${ClassName};
import ${packageName}.service.I${ClassName}Service;

/**
 * ${functionName}Service业务层处理
 *
 * @author ${author}
 * @date ${datetime}
 */
@Service
public class ${ClassName}ServiceImpl implements I${ClassName}Service
{
    @Autowired
    private ${ClassName}Mapper ${className}Mapper;

    /**
     * 查询${functionName}
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return ${functionName}
     */
    @Override
    public ${ClassName} select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField})
    {
        return ${className}Mapper.select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaField});
    }

    /**
     * 查询${functionName}列表
     *
     * @param ${className} ${functionName}
     * @return ${functionName}集合
     */
    @Override
    public List<${ClassName}> select${ClassName}List(${ClassName} ${className})
    {
        return ${className}Mapper.select${ClassName}List(${className});
    }

    /**
     * 新增${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
#if($table.sub)
    @Transactional
#end
    @Override
    public int insert${ClassName}(${ClassName} ${className})
    {
#foreach ($column in $columns)
#if($column.javaField == 'createTime')
        ${className}.setCreateTime(DateUtils.getNowDate());
#end
#end
#if($table.sub)
        int rows = ${className}Mapper.insert${ClassName}(${className});
        insert${subClassName}(${className});
        return rows;
#else
        return ${className}Mapper.insert${ClassName}(${className});
#end
    }

    /**
     * 修改${functionName}
     *
     * @param ${className} ${functionName}
     * @return 结果
     */
#if($table.sub)
    @Transactional
#end
    @Override
    public int update${ClassName}(${ClassName} ${className})
    {
#foreach ($column in $columns)
#if($column.javaField == 'updateTime')
        ${className}.setUpdateTime(DateUtils.getNowDate());
#end
#end
#if($table.sub)
        ${className}Mapper.delete${subClassName}By${subTableFkClassName}(${className}.get${pkColumn.capJavaField}());
        insert${subClassName}(${className});
#end
        return ${className}Mapper.update${ClassName}(${className});
    }

    /**
     * 批量删除${functionName}
     *
     * @param ${pkColumn.javaField}s 需要删除的${functionName}主键
     * @return 结果
     */
#if($table.sub)
    @Transactional
#end
    @Override
    public int delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaType}[] ${pkColumn.javaField}s)
    {
#if($table.sub)
        ${className}Mapper.delete${subClassName}By${subTableFkClassName}s(${pkColumn.javaField}s);
#end
        return ${className}Mapper.delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaField}s);
    }

    /**
     * 删除${functionName}信息
     *
     * @param ${pkColumn.javaField} ${functionName}主键
     * @return 结果
     */
#if($table.sub)
    @Transactional
#end
    @Override
    public int delete${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField})
    {
#if($table.sub)
        ${className}Mapper.delete${subClassName}By${subTableFkClassName}(${pkColumn.javaField});
#end
        return ${className}Mapper.delete${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaField});
    }
#if($table.sub)

    /**
     * 新增${subTable.functionName}信息
     *
     * @param ${className} ${functionName}对象
     */
    public void insert${subClassName}(${ClassName} ${className})
    {
        List<${subClassName}> ${subclassName}List = ${className}.get${subClassName}List();
        ${pkColumn.javaType} ${pkColumn.javaField} = ${className}.get${pkColumn.capJavaField}();
        if (StringUtils.isNotNull(${subclassName}List))
        {
            List<${subClassName}> list = new ArrayList<${subClassName}>();
            for (${subClassName} ${subclassName} : ${subclassName}List)
            {
                ${subclassName}.set${subTableFkClassName}(${pkColumn.javaField});
                list.add(${subclassName});
            }
            if (list.size() > 0)
            {
                ${className}Mapper.batch${subClassName}(list);
            }
        }
    }
#end
}
"#;

const OFFICIAL_DOMAIN_VM: &str = r#"package ${packageName}.domain;

#foreach ($import in $importList)
import ${import};
#end
import org.apache.commons.lang3.builder.ToStringBuilder;
import org.apache.commons.lang3.builder.ToStringStyle;
import com.ruoyi.common.annotation.Excel;
#if($table.crud || $table.sub)
import com.ruoyi.common.core.domain.BaseEntity;
#elseif($table.tree)
import com.ruoyi.common.core.domain.TreeEntity;
#end

/**
 * ${functionName}对象 ${tableName}
 *
 * @author ${author}
 * @date ${datetime}
 */
#if($table.crud || $table.sub)
#set($Entity="BaseEntity")
#elseif($table.tree)
#set($Entity="TreeEntity")
#end
public class ${ClassName} extends ${Entity}
{
    private static final long serialVersionUID = 1L;

#foreach ($column in $columns)
#if(!$table.isSuperColumn($column.javaField))
    /** $column.columnComment */
#if($column.list)
#set($parentheseIndex=$column.columnComment.indexOf("（"))
#if($parentheseIndex != -1)
#set($comment=$column.columnComment.substring(0, $parentheseIndex))
#else
#set($comment=$column.columnComment)
#end
#if($parentheseIndex != -1)
    @Excel(name = "${comment}", readConverterExp = "$column.readConverterExp()")
#elseif($column.javaType == 'Date')
    @JsonFormat(pattern = "yyyy-MM-dd")
    @Excel(name = "${comment}", width = 30, dateFormat = "yyyy-MM-dd")
#else
    @Excel(name = "${comment}")
#end
#end
    private $column.javaType $column.javaField;

#end
#end
#if($table.sub)
    /** $table.subTable.functionName信息 */
    private List<${subClassName}> ${subclassName}List;

#end
#foreach ($column in $columns)
#if(!$table.isSuperColumn($column.javaField))
#if($column.javaField.length() > 2 && $column.javaField.substring(1,2).matches("[A-Z]"))
#set($AttrName=$column.javaField)
#else
#set($AttrName=$column.javaField.substring(0,1).toUpperCase() + ${column.javaField.substring(1)})
#end
    public void set${AttrName}($column.javaType $column.javaField)
    {
        this.$column.javaField = $column.javaField;
    }

    public $column.javaType get${AttrName}()
    {
        return $column.javaField;
    }

#end
#end
#if($table.sub)
    public List<${subClassName}> get${subClassName}List()
    {
        return ${subclassName}List;
    }

    public void set${subClassName}List(List<${subClassName}> ${subclassName}List)
    {
        this.${subclassName}List = ${subclassName}List;
    }

#end
    @Override
    public String toString() {
        return new ToStringBuilder(this,ToStringStyle.MULTI_LINE_STYLE)
#foreach ($column in $columns)
#if($column.javaField.length() > 2 && $column.javaField.substring(1,2).matches("[A-Z]"))
#set($AttrName=$column.javaField)
#else
#set($AttrName=$column.javaField.substring(0,1).toUpperCase() + ${column.javaField.substring(1)})
#end
            .append("${column.javaField}", get${AttrName}())
#end
#if($table.sub)
            .append("${subclassName}List", get${subClassName}List())
#end
            .toString();
    }
}
"#;

const OFFICIAL_CONTROLLER_VM: &str = r#"package ${packageName}.controller;

import java.util.List;
import jakarta.servlet.http.HttpServletResponse;
import org.springframework.security.access.prepost.PreAuthorize;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.PutMapping;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import com.ruoyi.common.annotation.Log;
import com.ruoyi.common.core.controller.BaseController;
import com.ruoyi.common.core.domain.AjaxResult;
import com.ruoyi.common.enums.BusinessType;
import ${packageName}.domain.${ClassName};
import ${packageName}.service.I${ClassName}Service;
import com.ruoyi.common.utils.poi.ExcelUtil;
#if($table.crud || $table.sub)
import com.ruoyi.common.core.page.TableDataInfo;
#elseif($table.tree)
#end

/**
 * ${functionName}Controller
 *
 * @author ${author}
 * @date ${datetime}
 */
@RestController
@RequestMapping("/${moduleName}/${businessName}")
public class ${ClassName}Controller extends BaseController
{
    @Autowired
    private I${ClassName}Service ${className}Service;

    /**
     * 查询${functionName}列表
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:list')")
    @GetMapping("/list")
#if($table.crud || $table.sub)
    public TableDataInfo list(${ClassName} ${className})
    {
        startPage();
        List<${ClassName}> list = ${className}Service.select${ClassName}List(${className});
        return getDataTable(list);
    }
#elseif($table.tree)
    public AjaxResult list(${ClassName} ${className})
    {
        List<${ClassName}> list = ${className}Service.select${ClassName}List(${className});
        return success(list);
    }
#end

    /**
     * 导出${functionName}列表
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:export')")
    @Log(title = "${functionName}", businessType = BusinessType.EXPORT)
    @PostMapping("/export")
    public void export(HttpServletResponse response, ${ClassName} ${className})
    {
        List<${ClassName}> list = ${className}Service.select${ClassName}List(${className});
        ExcelUtil<${ClassName}> util = new ExcelUtil<${ClassName}>(${ClassName}.class);
        util.exportExcel(response, list, "${functionName}数据");
    }

    /**
     * 获取${functionName}详细信息
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:query')")
    @GetMapping(value = "/{${pkColumn.javaField}}")
    public AjaxResult getInfo(@PathVariable("${pkColumn.javaField}") ${pkColumn.javaType} ${pkColumn.javaField})
    {
        return success(${className}Service.select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaField}));
    }

    /**
     * 新增${functionName}
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:add')")
    @Log(title = "${functionName}", businessType = BusinessType.INSERT)
    @PostMapping
    public AjaxResult add(@RequestBody ${ClassName} ${className})
    {
        return toAjax(${className}Service.insert${ClassName}(${className}));
    }

    /**
     * 修改${functionName}
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:edit')")
    @Log(title = "${functionName}", businessType = BusinessType.UPDATE)
    @PutMapping
    public AjaxResult edit(@RequestBody ${ClassName} ${className})
    {
        return toAjax(${className}Service.update${ClassName}(${className}));
    }

    /**
     * 删除${functionName}
     */
    @PreAuthorize("@ss.hasPermi('${permissionPrefix}:remove')")
    @Log(title = "${functionName}", businessType = BusinessType.DELETE)
	@DeleteMapping("/{${pkColumn.javaField}s}")
    public AjaxResult remove(@PathVariable ${pkColumn.javaType}[] ${pkColumn.javaField}s)
    {
        return toAjax(${className}Service.delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaField}s));
    }
}
"#;

/// 从模板内容提取 `${receiver}方法名(` 形式的调用方法名（去重）。
/// receiver 如 `${className}Service.` / `${className}Mapper.`
fn extract_called_methods(content: &str, receiver: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in content.split(receiver).skip(1) {
        if let Some(end) = part.find('(') {
            let name = part[..end].trim().to_string();
            let valid = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '{' | '}'));
            if valid && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// 构造含 common/admin 模块 + 官方 generator 模板的合成项目根
fn build_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // common 模块 pom（含 dependencies 节点）
    write(
        root.join("ruoyi-common/pom.xml"),
        "<project>\n  <artifactId>ruoyi-common</artifactId>\n  <dependencies>\n  </dependencies>\n</project>\n",
    );
    write(
        root.join("ruoyi-admin/pom.xml"),
        "<project>\n  <artifactId>ruoyi-admin</artifactId>\n</project>\n",
    );
    // generator 模板（RuoYi 官方 vm/java 原文，自包含夹具）
    let vm = root.join("ruoyi-generator/src/main/resources/vm/java");
    write(vm.join("mapper.java.vm"), OFFICIAL_MAPPER_VM);
    write(vm.join("service.java.vm"), OFFICIAL_SERVICE_VM);
    write(vm.join("serviceImpl.java.vm"), OFFICIAL_SERVICE_IMPL_VM);
    write(vm.join("domain.java.vm"), OFFICIAL_DOMAIN_VM);
    write(vm.join("controller.java.vm"), OFFICIAL_CONTROLLER_VM);
    dir
}

#[test]
fn adds_mybatis_plus_dependency_idempotently() {
    let dir = build_project();
    let root = dir.path();
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];

    // 首次添加（合成项目无 parent 版本 → 默认 Boot 4 → boot4-starter + 现代档 jsqlparser）
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added, "首次应添加依赖");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-spring-boot4-starter"), "默认 Boot 4 应注入 boot4 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "默认 / Boot 4 应同时注入现代档 jsqlparser"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "默认 / Boot 4 不应注入 jsqlparser-4.9"
    );
    assert!(common_pom.contains("3.5.15"), "版本应为 3.5.15");

    // 再次添加应跳过（幂等：starter + jsqlparser 均已存在）
    let added2 = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(!added2, "两者都有时应跳过");
}

#[test]
fn selects_boot2_starter_when_parent_is_boot2() {
    let dir = build_project();
    let root = dir.path();
    // 根 pom 用 spring-boot-starter-parent 2.7.18（Spring 5 / Boot 2）
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>2.7.18</version>\n  </parent>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added);
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-boot-starter"), "Boot 2 应注入 boot2 starter");
    assert!(!common_pom.contains("boot3-starter"), "Boot 2 不应注入 boot3 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser-4.9</artifactId>"),
        "Boot 2 应注入 jsqlparser-4.9"
    );
    assert!(
        !common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "Boot 2 不应注入现代档 jsqlparser 精确标签"
    );
}

#[test]
fn selects_boot3_starter_when_parent_is_boot3() {
    let dir = build_project();
    let root = dir.path();
    // 根 pom 用 spring-boot-starter-parent 3.2.4（Spring 6 / Boot 3）
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.2.4</version>\n  </parent>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added);
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-spring-boot3-starter"), "Boot 3 应注入 boot3 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "Boot 3 应注入现代档 jsqlparser 精确标签"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "Boot 3 不应注入 jsqlparser-4.9"
    );
}

#[test]
fn backfills_jsqlparser_when_starter_already_present() {
    let dir = build_project();
    let root = dir.path();
    write(
        root.join("ruoyi-common/pom.xml"),
        "<project>\n  <artifactId>ruoyi-common</artifactId>\n  <dependencies>\n    <dependency>\n      <groupId>com.baomidou</groupId>\n      <artifactId>mybatis-plus-spring-boot4-starter</artifactId>\n      <version>3.5.15</version>\n    </dependency>\n  </dependencies>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added, "starter 已有但缺 jsqlparser 时应补依赖并返回 true");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "应补上现代档 jsqlparser"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "补依赖时不应写入 jsqlparser-4.9"
    );
    assert_eq!(
        common_pom.matches("mybatis-plus-spring-boot4-starter").count(),
        1,
        "不应重复写入 starter"
    );

    let added2 = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(!added2, "两者都有时应幂等返回 false");
}

/// Cloud 骨架：gateway + modules，job 无 datasource，system 已有。
fn build_cloud_mp_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-datasource/pom.xml"),
        "<project>\n  <parent>\n    <groupId>com.company.project</groupId>\n    <artifactId>ruoyi-common</artifactId>\n  </parent>\n  <artifactId>ruoyi-common-datasource</artifactId>\n  <dependencies></dependencies>\n</project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-system/pom.xml"),
        "<project><artifactId>ruoyi-system</artifactId><dependencies>\n        <dependency>\n            <groupId>com.company.project</groupId>\n            <artifactId>ruoyi-common-datasource</artifactId>\n        </dependency>\n</dependencies></project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies>\n</dependencies></project>\n",
    );
    dir
}

fn cloud_mp_params() -> CustomizeParams {
    CustomizeParams {
        new_package: "com.company.project".into(),
        new_module_prefix: "demo".into(),
        ..Default::default()
    }
}

fn cloud_mp_modules() -> Vec<String> {
    vec![
        "ruoyi-gateway".into(),
        "ruoyi-common/ruoyi-common-datasource".into(),
        "ruoyi-modules/ruoyi-system".into(),
        "ruoyi-modules/ruoyi-job".into(),
    ]
}

#[test]
fn cloud_job_gets_datasource_dep_idempotently() {
    let dir = build_cloud_mp_project();
    let root = dir.path();
    let modules = cloud_mp_modules();
    let params = cloud_mp_params();

    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 1, "仅 job 应补 common-datasource");

    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        job_pom.contains("<artifactId>ruoyi-common-datasource</artifactId>"),
        "job 应依赖读自 pom 的 artifactId：{job_pom}"
    );
    assert!(
        job_pom.contains("<groupId>com.company.project</groupId>"),
        "job 应使用 datasource pom 的 parent groupId：{job_pom}"
    );
    assert_eq!(
        job_pom.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1
    );
    assert!(
        !job_pom.contains("mybatis-plus-boot-starter")
            && !job_pom.contains("mybatis-plus-spring-boot3-starter")
            && !job_pom.contains("mybatis-plus-spring-boot4-starter"),
        "不要把 MP starter 再写一份到 job：{job_pom}"
    );

    let sys_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-system/pom.xml")).unwrap();
    assert_eq!(
        sys_pom.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1,
        "system 已有则不重复：{sys_pom}"
    );

    let n2 = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n2, 0, "再次执行应幂等跳过");
    let job_pom2 = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert_eq!(
        job_pom2.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1,
        "job 不应重复插入：{job_pom2}"
    );
}

#[test]
fn cloud_datasource_coords_fallback_to_params() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies></dependencies></project>\n",
    );
    let modules = vec![
        "ruoyi-gateway".into(),
        "ruoyi-modules/ruoyi-job".into(),
    ];
    let params = cloud_mp_params();
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 1);
    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        job_pom.contains("<artifactId>demo-common-datasource</artifactId>"),
        "读不到 datasource pom 时应回退 new_module_prefix：{job_pom}"
    );
    assert!(
        job_pom.contains("<groupId>com.company.project</groupId>"),
        "读不到 groupId 时应回退 new_package：{job_pom}"
    );
}

#[test]
fn cloud_skips_module_that_already_has_mp_starter() {
    let dir = build_cloud_mp_project();
    let root = dir.path();
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies>\n        <dependency>\n            <groupId>com.baomidou</groupId>\n            <artifactId>mybatis-plus-spring-boot4-starter</artifactId>\n        </dependency>\n</dependencies></project>\n",
    );
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(
        root,
        &cloud_mp_modules(),
        &cloud_mp_params(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(n, 0, "已含 MP starter 的 job 应跳过");
    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        !job_pom.contains("common-datasource"),
        "已有 starter 时不要再插 datasource：{job_pom}"
    );
}

#[test]
fn vue_layout_does_not_inject_datasource() {
    let dir = build_project();
    let root = dir.path();
    let params = cloud_mp_params();
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 0, "Vue 不应给模块补 common-datasource");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(
        !common_pom.contains("common-datasource"),
        "Vue common pom 不应被插入 datasource：{common_pom}"
    );
}

#[test]
fn detect_boot_major_version_reads_parent_version() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // 无 pom → None
    assert_eq!(mybatis_plus::detect_boot_major_version(root), None);
    // Boot 3 parent
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.5.0</version>\n  </parent>\n</project>\n",
    );
    assert_eq!(mybatis_plus::detect_boot_major_version(root), Some(3));
}

#[test]
fn generates_config_class_idempotently() {
    let dir = build_project();
    let root = dir.path();
    let params = CustomizeParams {
        new_package: "com.company.project".into(),
        ..Default::default()
    };
    let modules = vec!["ruoyi-admin".to_string()];

    let created = mybatis_plus::add_config_class(root, &params, &modules, &|_| {}).unwrap();
    assert!(created, "首次应生成配置类");
    let cfg = root.join("ruoyi-admin/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java");
    assert!(cfg.is_file(), "配置类应在正确包路径");
    let content = fs::read_to_string(&cfg).unwrap();
    assert!(content.contains("package com.company.project.framework.config"), "包名应为新包名");
    assert!(content.contains("MybatisPlusInterceptor"), "应含分页插件");
    assert!(content.contains("@Configuration"), "应有 @Configuration");

    // 再次生成应跳过
    let created2 = mybatis_plus::add_config_class(root, &params, &modules, &|_| {}).unwrap();
    assert!(!created2, "已存在应跳过");
}

#[test]
fn adapts_official_generator_templates_keeps_output_compilable() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/mapper.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/service.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/serviceImpl.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];

    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert!(n >= 4, "至少适配 4 个模板，实际 {n}");

    let mapper = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    let service = fs::read_to_string(root.join(&gen_files[1])).unwrap();
    let impl_ = fs::read_to_string(root.join(&gen_files[2])).unwrap();
    let domain = fs::read_to_string(root.join(&gen_files[3])).unwrap();
    // controller 模板不参与适配（方法都在 service 中保留后自然可编译）
    let controller = fs::read_to_string(
        root.join("ruoyi-generator/src/main/resources/vm/java/controller.java.vm"),
    )
    .unwrap();

    // 1. Service：继承 IService，且 controller 调用的每个方法都能在 service 模板中找到声明
    assert!(
        service.contains("extends IService<${ClassName}>"),
        "Service 应继承 IService：{service}"
    );
    assert!(
        service.contains("import com.baomidou.mybatisplus.extension.service.IService;"),
        "Service 应补 IService import：{service}"
    );
    let controller_calls = extract_called_methods(&controller, "${className}Service.");
    assert!(
        !controller_calls.is_empty(),
        "夹具 controller 应含 service 调用"
    );
    for m in &controller_calls {
        assert!(
            service.contains(&format!("{m}(")),
            "适配后 service 模板必须保留 {m} 声明（controller 仍在调用）：\n{service}"
        );
    }
    // 官方 service 六个方法声明逐一保留（不整文件替换）
    for decl in [
        "public ${ClassName} select${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});",
        "public List<${ClassName}> select${ClassName}List(${ClassName} ${className});",
        "public int insert${ClassName}(${ClassName} ${className});",
        "public int update${ClassName}(${ClassName} ${className});",
        "public int delete${ClassName}By${pkColumn.capJavaField}s(${pkColumn.javaType}[] ${pkColumn.javaField}s);",
        "public int delete${ClassName}By${pkColumn.capJavaField}(${pkColumn.javaType} ${pkColumn.javaField});",
    ] {
        assert!(service.contains(decl), "service 模板应保留声明：{decl}\n{service}");
    }

    // 2. ServiceImpl：继承 ServiceImpl，方法实现原样保留（含对 mapper 的调用）
    assert!(
        impl_.contains(
            "extends ServiceImpl<${ClassName}Mapper, ${ClassName}> implements I${ClassName}Service"
        ),
        "ServiceImpl 应改为继承 ServiceImpl 并实现 IService 接口：{impl_}"
    );
    assert!(
        impl_.contains("import com.baomidou.mybatisplus.extension.service.impl.ServiceImpl;"),
        "ServiceImpl 应补 ServiceImpl import：{impl_}"
    );
    assert_eq!(
        impl_.matches("@Override").count(),
        6,
        "六个业务方法的 @Override 实现应全部保留：{impl_}"
    );
    assert!(
        impl_.contains("return ${className}Mapper.select${ClassName}List(${className});"),
        "impl 方法体应原样保留：{impl_}"
    );

    // 3. Mapper：继承 BaseMapper，且 impl 调用的每个 mapper 方法仍有声明（否则生成代码不可编译）
    assert!(
        mapper.contains("extends BaseMapper<${ClassName}>"),
        "Mapper 应继承 BaseMapper：{mapper}"
    );
    assert!(
        mapper.contains("import com.baomidou.mybatisplus.core.mapper.BaseMapper;"),
        "Mapper 应补 BaseMapper import：{mapper}"
    );
    let mapper_calls = extract_called_methods(&impl_, "${className}Mapper.");
    assert!(!mapper_calls.is_empty(), "夹具 impl 应含 mapper 调用");
    for m in &mapper_calls {
        assert!(
            mapper.contains(&format!("{m}(")),
            "适配后 mapper 模板必须保留 {m} 声明（impl 方法体仍在调用）：\n{mapper}"
        );
    }

    // 4. Domain：@TableName + 主键 @TableId（$column.pk 条件） + Long 字段 Velocity 条件 @JsonSerialize（官方字段是 $column 变量）
    assert!(
        domain.contains("@TableName(\"${tableName}\")"),
        "Domain 应含 @TableName：{domain}"
    );
    assert!(
        domain.contains("#if($column.pk)\n    @TableId\n#end\n"),
        "主键字段前应注入 pk 条件 @TableId：{domain}"
    );
    assert!(
        domain.contains(
            "#if($column.javaType == 'Long')\n    @JsonSerialize(using = ToStringSerializer.class)\n#end\n    private $column.javaType $column.javaField;"
        ),
        "Long 字段行前应注入 Velocity 条件 @JsonSerialize 注解：\n{domain}"
    );
    assert!(
        domain.contains("import com.fasterxml.jackson.databind.annotation.JsonSerialize;"),
        "Domain 应补 JsonSerialize import：{domain}"
    );
    assert!(
        domain.contains("import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;"),
        "Domain 应补 ToStringSerializer import：{domain}"
    );

    // 幂等：再次适配应不改
    let n2 = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert_eq!(n2, 0, "已适配的模板再次执行应 0 改动");
}

/// 关闭 Long ID 开关时：domain 模板只加 @TableName，不注入条件序列化注解
#[test]
fn domain_template_without_long_id_skips_json_serialize() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];
    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, false, &|_| {}).unwrap();
    assert_eq!(n, 1);
    let domain = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    assert!(domain.contains("@TableName(\"${tableName}\")"));
    assert!(
        !domain.contains("@JsonSerialize"),
        "未开 Long ID 不应注入序列化注解：{domain}"
    );
}

// ---------- Domain 模板主键 @TableId 注入（官方夹具） ----------

/// 主键字段注入 @TableId 且被 #if($column.pk) 条件包裹（渲染时仅主键列输出，非主键不带）；
/// 与批B 的 @JsonSerialize 注入共存且顺序合理（@TableId 在前、序列化在后）。
#[test]
fn domain_template_pk_gets_tableid_others_dont() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];
    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert_eq!(n, 1);
    let domain = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    // 两个独立条件块紧贴字段声明行，#if/#end 各自配对，不破坏官方模板既有结构
    assert!(
        domain.contains(concat!(
            "#if($column.pk)\n",
            "    @TableId\n",
            "#end\n",
            "#if($column.javaType == 'Long')\n",
            "    @JsonSerialize(using = ToStringSerializer.class)\n",
            "#end\n",
            "    private $column.javaType $column.javaField;",
        )),
        "主键 Long 字段应同时带 @TableId 与 @JsonSerialize（@TableId 在前）：\n{domain}"
    );
    // 全模板仅一处 @TableId 注解：主键与非主键由 $column.pk 条件区分，不会落到非主键字段
    assert_eq!(
        domain.matches("@TableId").count(),
        1,
        "@TableId 注解只应有一处：\n{domain}"
    );
    // TableId import 与注解配套（批B 已注入，回归确认）
    assert!(
        domain.contains("import com.baomidou.mybatisplus.annotation.TableId;"),
        "{domain}"
    );
}

/// 关闭 Long ID 开关：主键 @TableId 仍注入（MP 适配不依赖序列化开关），序列化注解不注入
#[test]
fn domain_template_tableid_present_without_long_id() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];
    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, false, &|_| {}).unwrap();
    assert_eq!(n, 1);
    let domain = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    assert!(
        domain.contains("#if($column.pk)\n    @TableId\n#end\n"),
        "MP 适配下主键 @TableId 不依赖 Long ID 开关：\n{domain}"
    );
    assert!(!domain.contains("@JsonSerialize"), "{domain}");
}

/// 渲染正确性：注入的 import 必须位于 `#foreach ($import in $importList)` 循环体之外。
/// 官方 VelocityUtils.getImportList 在表无 Date/BigDecimal 字段且非子表时返回空集合，
/// 循环体渲染 0 次——import 落在循环内会随空循环整体消失，生成代码缺 import 编译失败
/// （旧版锚定「首个 import 行」即循环体内的 `import ${import};`，已修正为首个字面量 import 行）。
#[test]
fn domain_template_imports_render_outside_import_foreach() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];
    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert_eq!(n, 1);
    let domain = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    assert!(
        domain.contains(
            "#foreach ($import in $importList)\nimport ${import};\n#end\nimport com.baomidou.mybatisplus.annotation.TableName;"
        ),
        "import 应注入在 import foreach 循环之后（首个字面量 import 前）：\n{domain}"
    );
    assert!(
        !domain.contains("#foreach ($import in $importList)\nimport com.baomidou"),
        "import 不得注入 import foreach 循环体内：\n{domain}"
    );
}

/// 雪花语义协同：适配注入 plain @TableId 后，snowflake::mark_domain_idtype_input
/// （executor 雪花任务 do_inject_snowflake_id 步骤 4a 对生成器 domain 模板调用的同一函数）
/// 把已有 @TableId 升级为 IdType.INPUT 并补 import——与存量 domain 源码
/// 「先有 @TableId、再被改写」同函数同语义（插入前由雪花工具 setId）。
#[test]
fn domain_template_tableid_upgraded_to_idtype_input_by_snowflake_mark() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];
    mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    let adapted = fs::read_to_string(root.join(&gen_files[0])).unwrap();

    // 雪花开形态：plain @TableId 被改写为 IdType.INPUT
    let upgraded = match snowflake::mark_domain_idtype_input(&adapted) {
        Some(u) => u,
        None => panic!("适配后的模板应可被雪花 mark 升级：\n{adapted}"),
    };
    assert!(
        upgraded.contains("#if($column.pk)\n    @TableId(type = IdType.INPUT)\n#end\n"),
        "主键 @TableId 应升级为 IdType.INPUT：\n{upgraded}"
    );
    assert!(
        upgraded.contains("import com.baomidou.mybatisplus.annotation.IdType;"),
        "应补 IdType import：\n{upgraded}"
    );
    // IdType import 同样落在 import foreach 之外（空 $importList 时不随循环消失）
    assert!(
        upgraded.contains(
            "#end\nimport com.baomidou.mybatisplus.annotation.IdType;\nimport com.baomidou.mybatisplus.annotation.TableName;"
        ),
        "IdType import 应位于循环外、其余注入 import 之前：\n{upgraded}"
    );
    // @JsonSerialize 块原样保留：两个注解共存，互不吞掉
    assert!(upgraded.contains(
        "#if($column.javaType == 'Long')\n    @JsonSerialize(using = ToStringSerializer.class)\n#end"
    ));
    assert_eq!(upgraded.matches("@TableId").count(), 1, "{upgraded}");
    // 幂等：再次 mark 不再改动；MP 层二次适配也 0 改动
    assert!(snowflake::mark_domain_idtype_input(&upgraded).is_none());
    let n2 = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert_eq!(n2, 0, "已适配的模板再次执行应 0 改动");
}

// ---------- 管线级：识别 → 规划 → 执行（官方模板夹具贯穿全链路） ----------

/// 可识别的合成 RuoYi-Vue 骨架（镜像 tests/execute_core.rs 的 build_fake_project）
/// + 官方生成器模板夹具：供 planner / executor 管线级用例验证 domain 模板注入组合。
fn build_pipeline_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n</modules>\n</project>\n",
    );
    for m in ["admin", "common", "framework", "system", "generator"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(
            mod_dir.join("pom.xml"),
            "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-m</artifactId>\n</project>\n",
        );
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        write(
            pkg_dir.join("UserService.java"),
            "package com.ruoyi.user;\nimport com.ruoyi.common.Util;\npublic class UserService {}\n",
        );
    }
    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(ui.join("src/views/login.vue"), "<template><div>若依后台管理系统</div></template>\n");
    write(ui.join("package.json"), "{\"name\":\"ruoyi\",\"title\":\"若依管理系统\"}");
    write(
        ui.join(".env.development"),
        "# 页面标题\nVUE_APP_TITLE = 若依管理系统\n\n# 开发环境配置\nENV = 'development'\n\n# 若依管理系统/开发环境\nVUE_APP_BASE_API = '/dev-api'\n",
    );
    // 官方生成器模板（与 build_project 相同夹具）
    let vm = root.join("ruoyi-generator/src/main/resources/vm/java");
    write(vm.join("mapper.java.vm"), OFFICIAL_MAPPER_VM);
    write(vm.join("service.java.vm"), OFFICIAL_SERVICE_VM);
    write(vm.join("serviceImpl.java.vm"), OFFICIAL_SERVICE_IMPL_VM);
    write(vm.join("domain.java.vm"), OFFICIAL_DOMAIN_VM);
    write(vm.join("controller.java.vm"), OFFICIAL_CONTROLLER_VM);
    dir
}

/// 管线用参数：只开指定开关，其余增强项全关，保证断言聚焦 MP / 雪花链路
fn pipeline_params(mp: bool, gen_mp: bool, long_id: bool, snowflake: bool) -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "测试系统".into(),
        enable_mybatis_plus: mp,
        enable_generator_mybatis_plus: gen_mp,
        enable_long_id_json_string: long_id,
        enable_snowflake_id: snowflake,
        enable_config_rewrite: false,
        enable_logback_rewrite: false,
        enable_report: false,
        enable_delivery_doc: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        enable_ai_rules: false,
        enable_footer_icp: false,
        enable_site_settings: false,
        ..Default::default()
    }
}

/// 跑完整管线（识别 → 规划 → 执行），断言无任务失败，返回项目目录与改造后的 domain 模板路径
fn run_pipeline(params: &CustomizeParams) -> (tempfile::TempDir, PathBuf) {
    let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    let template =
        ruoyi_forge_lib::rules::template::TemplateSet::load_from_dir(&template_dir)
            .unwrap()
            .into_full_template()
            .unwrap();
    let dir = build_pipeline_project();
    let root = dir.path();
    let info = ruoyi_forge_lib::core::detector::detect(root, &template);
    assert!(info.confidence.recognized, "合成骨架应识别为 RuoYi-Vue");
    assert!(
        !info.generator_template_files.is_empty(),
        "官方生成器模板应被识别"
    );
    let tasks = ruoyi_forge_lib::core::planner::plan(&info, params, &template);
    let results =
        ruoyi_forge_lib::core::executor::execute_all(root, &info, &tasks, params, &template, |_| {});
    for r in &results {
        assert!(
            !matches!(r.status, ruoyi_forge_lib::core::task::TaskStatus::Failed),
            "任务 {} 失败：{}",
            r.task_name,
            r.message
        );
    }
    // generator 模块已随核心任务重命名为 demo-generator
    let domain_vm = root.join("demo-generator/src/main/resources/vm/java/domain.java.vm");
    assert!(domain_vm.is_file(), "domain.java.vm 应存在于重命名后的模块");
    (dir, domain_vm)
}

/// 管线级闭环：MP + 生成器适配 + Long ID + 雪花全开——
/// 任务 10/11 注入 pk 条件 plain @TableId 与 Long 条件 @JsonSerialize，
/// 任务 11b（雪花 + MP 同开）把已有 @TableId 统一升级为 IdType.INPUT 并补 IdType import。
#[test]
fn pipeline_domain_template_tableid_composition_with_snowflake() {
    let params = pipeline_params(true, true, true, true);
    let (_dir, domain_vm) = run_pipeline(&params);
    let domain = fs::read_to_string(&domain_vm).unwrap();
    assert!(
        domain.contains("#if($column.pk)\n    @TableId(type = IdType.INPUT)\n#end\n"),
        "雪花 + MP 同开：主键应升级为 IdType.INPUT：\n{domain}"
    );
    assert!(
        domain.contains("import com.baomidou.mybatisplus.annotation.IdType;"),
        "应补 IdType import：\n{domain}"
    );
    // @JsonSerialize 与 @TableId 共存，互不吞掉
    assert!(domain.contains(
        "#if($column.javaType == 'Long')\n    @JsonSerialize(using = ToStringSerializer.class)\n#end\n    private $column.javaType $column.javaField;"
    ));
    // import 位于 import foreach 之外（空 $importList 时仍渲染）
    assert!(
        domain.contains(
            "#foreach ($import in $importList)\nimport ${import};\n#end\nimport com.baomidou.mybatisplus.annotation.IdType;"
        ),
        "注入 import 应位于 import foreach 循环体之外：\n{domain}"
    );
}

/// 管线级开关矩阵：
/// 1. MP 相关开关全关 → domain 模板零注入（关闭 enable_mybatis_plus 不注入 @TableId）
/// 2. 仅关 enable_mybatis_plus（生成器适配仍开）→ 保留 plain @TableId，
///    雪花开启也不升级 IdType.INPUT（executor 11b 的标记步骤以 enable_mybatis_plus 为门，
///    IdType.INPUT 属 MP 注解语义；生成器模板适配由 enable_generator_mybatis_plus 独立门控为既有行为）
#[test]
fn pipeline_domain_template_off_matrix_without_mp() {
    // 1. 全关：模板零注入
    let params = pipeline_params(false, false, false, false);
    let (_dir, domain_vm) = run_pipeline(&params);
    let domain = fs::read_to_string(&domain_vm).unwrap();
    assert!(!domain.contains("@TableId"), "{domain}");
    assert!(!domain.contains("@TableName"), "{domain}");
    assert!(!domain.contains("@JsonSerialize"), "{domain}");
    assert!(!domain.contains("IdType"), "{domain}");

    // 2. 仅关 enable_mybatis_plus：plain @TableId 保留、无 IdType.INPUT
    let params = pipeline_params(false, true, true, true);
    let (_dir, domain_vm) = run_pipeline(&params);
    let domain = fs::read_to_string(&domain_vm).unwrap();
    assert!(
        domain.contains("#if($column.pk)\n    @TableId\n#end\n"),
        "MP 关闭 + 生成器适配开启：主键应为 plain @TableId：\n{domain}"
    );
    assert!(!domain.contains("IdType.INPUT"), "{domain}");
}
