import argparse
import os


def main():
    parser = argparse.ArgumentParser(description="输出一句问候。")
    parser.add_argument("name", nargs="?", default="世界", help="问候对象（默认：世界）")
    parser.add_argument("--prefix", default=os.getenv("DEMO_PREFIX", "你好"),
                        help="问候语（默认：环境变量 DEMO_PREFIX 或 你好）")
    args = parser.parse_args()
    print(f"{args.prefix}，{args.name}！")


if __name__ == "__main__":
    main()
