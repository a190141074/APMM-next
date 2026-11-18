"""从应用的受信资源清单生成服务分类索引，不读取作者上传的映射。"""
import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output",type=Path,required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
names = {"cursor_up","cursor_down"}
for path in [root/"config/manifest.json", root/"config/resources.json", *sorted((root/"config").glob("*manifest*.json"))]:
    data=json.loads(path.read_text(encoding="utf-8-sig"))
    if isinstance(data,list):
        names.update(row["canonical"] for row in data if isinstance(row,dict) and "canonical" in row)
args.output.parent.mkdir(parents=True,exist_ok=True)
args.output.write_text(json.dumps(sorted(names),ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print(f"已生成 {len(names)} 个受信资源名称")
