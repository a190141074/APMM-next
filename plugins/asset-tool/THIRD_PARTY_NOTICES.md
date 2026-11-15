# 第三方组件

本工具调用下列开源组件。原始许可保存在 `THIRD_PARTY_LICENSES/`，发行程序旁包含同一份许可目录。工具自身的作者和许可由项目所有者确定，本说明不替代各组件原始许可。

| 组件 | 用途 |
| --- | --- |
| Python 3.12.14 | 独立发行包的解释器，包含 Python 原始许可。 |
| UnityPy 1.25.4 | Unity 序列化文件和类型树读取。 |
| texture2ddecoder 1.0.6 | 原生 BC7 等纹理解码。 |
| Pillow 12.3.0 | 图像转换和 PNG 编码。 |
| tpk_ar 0.2.4 | UnityPy 配套组件；低资源运行不加载完整类型资料库。 |
| lz4 4.4.5 | UnityFS LZ4 解压。 |
| brotli、etcpak、astc-encoder-py | UnityPy 随附的格式支持。 |
| attrs、archspec、fsspec | 配套类型和运行支持。 |
| fmod_toolkit、pyfmodex | 开发依赖中的可选音频支持；独立发行版排除这两个模块。 |
| PyInstaller 6.22.3 | 独立程序打包；原许可含打包程序分发例外。 |
| pyinstaller-hooks-contrib、pywin32-ctypes | Windows 打包支持。 |
| pefile、packaging、altgraph、setuptools | 构建和依赖元数据支持。 |

公共字段名表由已锁定 UnityPy/AssetRipper TPK 的标准表生成，只保存字段名称与偏移。完整依赖版本记录在 `requirements.lock`。

游戏图片属于其原权利人；本机导出的图片是用户本地结果，不纳入源码依赖或通用程序发行包。
