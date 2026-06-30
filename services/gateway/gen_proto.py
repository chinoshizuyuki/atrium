"""Generate gRPC stubs from proto definition."""

import subprocess
import sys
from pathlib import Path

PROTO_DIR = Path(__file__).resolve().parent.parent.parent / "proto"
OUT_DIR = Path(__file__).resolve().parent / "atrium" / "proto"

OUT_DIR.mkdir(parents=True, exist_ok=True)

# 确保 proto 目录是合法 Python 包 / Ensure proto dir is a proper Python package
init_file = OUT_DIR / "__init__.py"
if not init_file.exists():
    init_file.write_text('"""Generated gRPC stubs / gRPC 生成文件"""\n', encoding="utf-8")

proto_file = PROTO_DIR / "atrium.proto"
if not proto_file.exists():
    raise FileNotFoundError(f"Proto file not found: {proto_file}")

subprocess.run(
    [
        sys.executable, "-m", "grpc_tools.protoc",
        f"--proto_path={PROTO_DIR}",
        f"--python_out={OUT_DIR}",
        f"--grpc_python_out={OUT_DIR}",
        str(proto_file),
    ],
    check=True,
)

# Fix relative import in generated grpc file
grpc_file = OUT_DIR / "atrium_pb2_grpc.py"
content = grpc_file.read_text(encoding="utf-8")
content = content.replace(
    'import atrium_pb2 as atrium__pb2',
    'from . import atrium_pb2 as atrium__pb2',
)
grpc_file.write_text(content, encoding="utf-8")

print(f"gRPC stubs generated in {OUT_DIR}")