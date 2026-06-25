import requests
from prefect import flow, get_run_logger, task
from prefect.blocks.system import Secret

# ═══════════════════════════════════════════════════════════════════════════════
# Bank Rate Sync — Prefect Workflow
# ═══════════════════════════════════════════════════════════════════════════════
#
# Luồng:
#   1. Lấy OAuth2 token từ Auth0 (dùng chung config với gold flow)
#   2. Gọi PowerBI API → prefetch dữ liệu bank rate Online mới nhất
#   3. Parse response → list rows {bank, rate_1..rate_12}
#   4. Push từng row lên bảng admin qua POST /v1/seo/tables/{id}
#
# ───────────────────────────────────────────────────────────────────────────────
# Tạo bảng bank_rates lần đầu (cần token từ gold flow):
#
#   TOKEN="..."  # lấy từ gold flow hoặc curl thủ công
#   curl -s --request POST \
#     "$API_BASE/v1/seo/tables" \
#     -H "Authorization: Bearer $TOKEN" \
#     -H 'Content-Type: application/json;charset=UTF-8' \
#     -d '[
#       {
#         "table": "bank_rates",
#         "backend": "Rdbms",
#         "columns": [
#           {"name": "bank",  "kind": "Text"},
#           {"name": "rate_1",  "kind": "Text"},
#           {"name": "rate_3",  "kind": "Text"},
#           {"name": "rate_6",  "kind": "Text"},
#           {"name": "rate_9",  "kind": "Text"},
#           {"name": "rate_12", "kind": "Text"}
#         ]
#       }
#     ]'
#
# Response trả về id — dùng làm bank-rate-table-id bên dưới.
#
# ───────────────────────────────────────────────────────────────────────────────
# Prefect Secrets cần tạo:
#
#   1. findaily-upsert-client-secret  (đã có từ gold flow)
#   2. findaily-upsert-client-id      (đã có từ gold flow)
#   3. findaily-upsert-audience       (đã có từ gold flow)
#   4. findaily-upsert-api-base       (đã có từ gold flow)
#   5. bank-rate-table-id             (tạo mới: prefect secret set ...)
#
# Cách chạy:
#   cd pkgs/workflow
#   uv run python bank_rate.py                    # chạy 1 lần
#   uv run python main.py                         # serve với schedule (8h sáng)
# ═══════════════════════════════════════════════════════════════════════════════

# ─── CONFIG LOADER ───────────────────────────────────────────────────────────


def load_config():
    """
    Nạp cấu hình an toàn.
    Các thông tin nhạy cảm được lấy từ Prefect Secret Blocks.
    Các thông tin định danh/URL được lấy từ Env hoặc gán mặc định.
    """
    try:
        client_secret = Secret.load("findaily-upsert-client-secret").get()
        client_id = Secret.load("findaily-upsert-client-id").get()
        audience = Secret.load("findaily-upsert-audience").get()
        api_base = Secret.load("findaily-upsert-api-base").get()

        table_id = Secret.load("bank-rate-table-id").get()
    except Exception as e:
        raise e

    return {
        "AUTH0_URL": "https://universal-lazycat-auth.us.auth0.com/oauth/token",
        "CLIENT_ID": client_id,
        "CLIENT_SECRET": client_secret,
        "AUDIENCE": audience,
        "API_BASE": api_base,
        "TABLE_ID": table_id,
    }


# Khởi tạo config một lần
CONFIG = load_config()

# ─── POWERBI CONSTANTS ───────────────────────────────────────────────────────

POWERBI_URL = (
    "https://wabi-south-east-asia-api.analysis.windows.net/public/reports/querydata"
    "?synchronous=true"
)

POWERBI_USER_AGENT = (
    "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Mobile Safari/537.36"
)

# Column mapping: index trong mảng C -> tên cột
COLUMN_MAP = ["bank", "rate_1", "rate_3", "rate_6", "rate_9", "rate_12"]


def build_powerbi_headers(resource_key: str) -> dict:
    """Tạo headers cho PowerBI API."""
    return {
        "accept": "application/json, text/plain, */*",
        "accept-language": "en-US,en;q=0.9",
        "content-type": "application/json;charset=UTF-8",
        "origin": "https://app.powerbi.com",
        "referer": "https://app.powerbi.com/",
        "x-powerbi-resourcekey": resource_key,
        "user-agent": POWERBI_USER_AGENT,
    }


def build_powerbi_payload() -> dict:
    """
    Tạo payload query PowerBI.

    Query này dùng subquery để PREFETCH ngày báo cáo mới nhất
    (lọc channel='Online', datereport trong 15 tháng gần đây,
    lấy TOP 1 theo MAX(datereport) DESC), sau đó dùng kết quả đó
    để filter dữ liệu chính.
    """
    return {
        "version": "1.0.0",
        "queries": [
            {
                "Query": {
                    "Commands": [
                        {
                            "SemanticQueryDataShapeCommand": {
                                "Query": {
                                    "Version": 2,
                                    "From": [
                                        {
                                            "Name": "b",
                                            "Entity": "Bank Rate",
                                            "Type": 0,
                                        },
                                        {
                                            "Name": "subquery",
                                            "Expression": {
                                                "Subquery": {
                                                    "Query": {
                                                        "Version": 2,
                                                        "From": [
                                                            {
                                                                "Name": "b1",
                                                                "Entity": "Bank Rate",
                                                                "Type": 0,
                                                            }
                                                        ],
                                                        "Select": [
                                                            {
                                                                "Column": {
                                                                    "Expression": {
                                                                        "SourceRef": {
                                                                            "Source": "b1"
                                                                        }
                                                                    },
                                                                    "Property": "datereport",
                                                                },
                                                                "Name": "field",
                                                            }
                                                        ],
                                                        "Where": [
                                                            {
                                                                "Condition": {
                                                                    "Comparison": {
                                                                        "ComparisonKind": 0,
                                                                        "Left": {
                                                                            "Column": {
                                                                                "Expression": {
                                                                                    "SourceRef": {
                                                                                        "Source": "b1"
                                                                                    }
                                                                                },
                                                                                "Property": "bank",
                                                                            }
                                                                        },
                                                                        "Right": {
                                                                            "AnyValue": {
                                                                                "DefaultValueOverridesAncestors": True
                                                                            }
                                                                        },
                                                                    }
                                                                }
                                                            },
                                                            {
                                                                "Condition": {
                                                                    "In": {
                                                                        "Expressions": [
                                                                            {
                                                                                "Column": {
                                                                                    "Expression": {
                                                                                        "SourceRef": {
                                                                                            "Source": "b1"
                                                                                        }
                                                                                    },
                                                                                    "Property": "channel",
                                                                                }
                                                                            }
                                                                        ],
                                                                        "Values": [
                                                                            [
                                                                                {
                                                                                    "Literal": {
                                                                                        "Value": "'Online'"
                                                                                    }
                                                                                }
                                                                            ]
                                                                        ],
                                                                    }
                                                                }
                                                            },
                                                            {
                                                                "Condition": {
                                                                    "Between": {
                                                                        "Expression": {
                                                                            "Column": {
                                                                                "Expression": {
                                                                                    "SourceRef": {
                                                                                        "Source": "b1"
                                                                                    }
                                                                                },
                                                                                "Property": "datereport",
                                                                            }
                                                                        },
                                                                        "LowerBound": {
                                                                            "DateSpan": {
                                                                                "Expression": {
                                                                                    "DateAdd": {
                                                                                        "Expression": {
                                                                                            "DateAdd": {
                                                                                                "Expression": {
                                                                                                    "Now": {}
                                                                                                },
                                                                                                "Amount": 1,
                                                                                                "TimeUnit": 0,
                                                                                            }
                                                                                        },
                                                                                        "Amount": -15,
                                                                                        "TimeUnit": 2,
                                                                                    }
                                                                                },
                                                                                "TimeUnit": 0,
                                                                            }
                                                                        },
                                                                        "UpperBound": {
                                                                            "DateSpan": {
                                                                                "Expression": {
                                                                                    "Now": {}
                                                                                },
                                                                                "TimeUnit": 0,
                                                                            }
                                                                        },
                                                                    }
                                                                }
                                                            },
                                                        ],
                                                        "OrderBy": [
                                                            {
                                                                "Direction": 2,
                                                                "Expression": {
                                                                    "Aggregation": {
                                                                        "Expression": {
                                                                            "Column": {
                                                                                "Expression": {
                                                                                    "SourceRef": {
                                                                                        "Source": "b1"
                                                                                    }
                                                                                },
                                                                                "Property": "datereport",
                                                                            }
                                                                        },
                                                                        "Function": 4,
                                                                    }
                                                                },
                                                            }
                                                        ],
                                                        "Top": 1,
                                                    }
                                                }
                                            },
                                            "Type": 2,
                                        },
                                    ],
                                    "Select": [
                                        {
                                            "Column": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "bank",
                                            },
                                            "Name": "Bank Rate.bank",
                                        },
                                        {
                                            "Measure": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "rate_1",
                                            },
                                            "Name": "Bank Rate.rate_1",
                                        },
                                        {
                                            "Measure": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "rate_3",
                                            },
                                            "Name": "Bank Rate.rate_3",
                                        },
                                        {
                                            "Measure": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "rate_6",
                                            },
                                            "Name": "Bank Rate.rate_6",
                                        },
                                        {
                                            "Measure": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "rate_9",
                                            },
                                            "Name": "Bank Rate.rate_9",
                                        },
                                        {
                                            "Measure": {
                                                "Expression": {
                                                    "SourceRef": {"Source": "b"}
                                                },
                                                "Property": "rate_12",
                                            },
                                            "Name": "Bank Rate.rate_12",
                                        },
                                    ],
                                    "Where": [
                                        {
                                            "Condition": {
                                                "In": {
                                                    "Expressions": [
                                                        {
                                                            "Column": {
                                                                "Expression": {
                                                                    "SourceRef": {
                                                                        "Source": "b"
                                                                    }
                                                                },
                                                                "Property": "datereport",
                                                            }
                                                        }
                                                    ],
                                                    "Table": {
                                                        "SourceRef": {
                                                            "Source": "subquery"
                                                        }
                                                    },
                                                }
                                            }
                                        },
                                        {
                                            "Condition": {
                                                "In": {
                                                    "Expressions": [
                                                        {
                                                            "Column": {
                                                                "Expression": {
                                                                    "SourceRef": {
                                                                        "Source": "b"
                                                                    }
                                                                },
                                                                "Property": "channel",
                                                            }
                                                        }
                                                    ],
                                                    "Values": [
                                                        [
                                                            {
                                                                "Literal": {
                                                                    "Value": "'Online'"
                                                                }
                                                            }
                                                        ]
                                                    ],
                                                }
                                            }
                                        },
                                        {
                                            "Condition": {
                                                "Between": {
                                                    "Expression": {
                                                        "Column": {
                                                            "Expression": {
                                                                "SourceRef": {
                                                                    "Source": "b"
                                                                }
                                                            },
                                                            "Property": "datereport",
                                                        }
                                                    },
                                                    "LowerBound": {
                                                        "DateSpan": {
                                                            "Expression": {
                                                                "DateAdd": {
                                                                    "Expression": {
                                                                        "DateAdd": {
                                                                            "Expression": {
                                                                                "Now": {}
                                                                            },
                                                                            "Amount": 1,
                                                                            "TimeUnit": 0,
                                                                        }
                                                                    },
                                                                    "Amount": -15,
                                                                    "TimeUnit": 2,
                                                                }
                                                            },
                                                            "TimeUnit": 0,
                                                        }
                                                    },
                                                    "UpperBound": {
                                                        "DateSpan": {
                                                            "Expression": {"Now": {}},
                                                            "TimeUnit": 0,
                                                        }
                                                    },
                                                }
                                            }
                                        },
                                    ],
                                    "OrderBy": [
                                        {
                                            "Direction": 1,
                                            "Expression": {
                                                "Column": {
                                                    "Expression": {
                                                        "SourceRef": {"Source": "b"}
                                                    },
                                                    "Property": "bank",
                                                }
                                            },
                                        }
                                    ],
                                },
                                "Binding": {
                                    "Primary": {
                                        "Groupings": [
                                            {"Projections": [0, 1, 2, 3, 4, 5]}
                                        ]
                                    },
                                    "DataReduction": {
                                        "DataVolume": 3,
                                        "Primary": {"Window": {"Count": 500}},
                                    },
                                    "Version": 1,
                                },
                                "ExecutionMetricsKind": 1,
                            }
                        }
                    ]
                },
                "CacheKey": "...",
                "QueryId": "",
                "ApplicationContext": {
                    "DatasetId": "3b3e652e-64b1-46a8-9c04-eb7d00eb9413",
                    "Sources": [
                        {
                            "ReportId": "5a136594-c183-428a-b977-6d4f8c57abfe",
                            "VisualId": "9338a754be742818de8e",
                        }
                    ],
                },
            }
        ],
        "cancelQueries": [],
        "modelId": 7245784,
    }


# ─── TASKS ───────────────────────────────────────────────────────────────────


@task(retries=3, retry_delay_seconds=10)
def get_token():
    """Lấy token từ Auth0 để xác thực API (dùng chung với gold flow)."""
    if not CONFIG["CLIENT_SECRET"]:
        raise ValueError("CLIENT_SECRET is missing! Check Prefect Blocks or Env.")

    resp = requests.post(
        CONFIG["AUTH0_URL"],
        json={
            "client_id": CONFIG["CLIENT_ID"],
            "client_secret": CONFIG["CLIENT_SECRET"],
            "audience": CONFIG["AUDIENCE"],
            "grant_type": "client_credentials",
        },
        timeout=15,
    )
    resp.raise_for_status()
    return resp.json()["access_token"]


@task(retries=2, retry_delay_seconds=30)
def prefetch_bank_rates_raw(resource_key: str) -> dict:
    """
    Task 1 — PREFETCH: Gọi PowerBI API để lấy dữ liệu bank rate.

    Query có built-in subquery (prefetch) để tự động lọc ra
    ngày báo cáo mới nhất cho mỗi ngân hàng, channel Online.
    """
    logger = get_run_logger()
    logger.info("📡 Prefetch dữ liệu Bank Rate từ PowerBI...")

    headers = build_powerbi_headers(resource_key)
    payload = build_powerbi_payload()

    resp = requests.post(
        POWERBI_URL,
        json=payload,
        headers=headers,
        timeout=60,
    )
    resp.raise_for_status()

    logger.info(f"   ✅ PowerBI response OK ({len(resp.content)} bytes)")
    return resp.json()


@task(retries=2, retry_delay_seconds=10)
def parse_bank_rates(raw_data: dict) -> list[dict]:
    """
    Task 2 — PARSE: Parse response PowerBI thành list of rows.

    Cấu trúc response:
      results[0].result.data.dsr.DS[].PH[].DM1[]
      Mỗi DM1 entry có:
        - "C": [bank, rate_1, rate_3, rate_6, rate_9, rate_12]
        - "S": schema (chỉ entry đầu tiên)
        - "R": số lượng giá trị bị truncate (nếu có)
    """
    logger = get_run_logger()
    logger.info("📊 Parse dữ liệu Bank Rate...")

    try:
        ds_list = raw_data["results"][0]["result"]["data"]["dsr"]["DS"]
    except (KeyError, IndexError, TypeError) as exc:
        raise ValueError(f"Không thể parse response PowerBI: {exc}") from exc

    rows = []
    total_entries = 0
    truncated_count = 0

    for ds in ds_list:
        ph_list = ds.get("PH", [])
        for ph in ph_list:
            dm1 = ph.get("DM1", [])
            for entry in dm1:
                total_entries += 1
                values = entry.get("C", [])
                remaining = entry.get("R", 0)

                if not values or len(values) < 1:
                    continue

                row = {}
                for i, col_name in enumerate(COLUMN_MAP):
                    if i < len(values):
                        val = values[i]
                        row[col_name] = str(val) if val is not None else ""
                    elif remaining > 0:
                        row[col_name] = "N/A"
                    else:
                        row[col_name] = ""

                if remaining > 0:
                    truncated_count += 1
                    logger.warning(
                        f"   ⚠️  {row['bank']}: {len(values) - 1}/5 rates "
                        f"(còn {remaining} giá trị bị truncate)"
                    )

                rows.append(row)

    logger.info(
        f"   ✅ Parse xong: {len(rows)} banks "
        f"({truncated_count} bị truncate) / {total_entries} entries"
    )

    if not rows:
        raise Exception("Không có dữ liệu bank rate nào được parse!")

    return rows


@task(retries=0, retry_delay_seconds=5, tags=["admin-api-limit"])
def push_bank_rate(token: str, table_id: str, row: dict) -> dict:
    """
    Task 3 — PUSH: Đẩy một row bank rate lên bảng admin.

    API: POST /v1/seo/tables/{table_id}
    Auth: OAuth2 Bearer token (dùng chung Auth0 config với gold flow)
    """
    logger = get_run_logger()
    bank_name = row.get("bank", "UNKNOWN")

    url = f"{CONFIG['API_BASE']}/v1/seo/tables/{table_id}"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json;charset=UTF-8",
    }

    try:
        resp = requests.post(url, json=row, headers=headers, timeout=30)
        resp.raise_for_status()
        logger.info(f"   ✅ {bank_name}")
        return {"bank": bank_name, "status": "ok"}
    except requests.RequestException as exc:
        error_detail = (
            f"HTTP {resp.status_code}: {resp.text[:200]}"
            if hasattr(resp, "status_code")
            else str(exc)
        )
        logger.error(f"   ❌ {bank_name}: {error_detail}")
        return {"bank": bank_name, "status": "error", "error": error_detail}


# ─── FLOW ────────────────────────────────────────────────────────────────────


@flow(
    name="Bank-Rate-Sync",
    description="Đồng bộ lãi suất ngân hàng (Bank Rate) từ PowerBI vào bảng admin",
)
def bank_rate_sync_flow(resource_key: str = "fc45cd6a-2c7e-4c4d-99ae-4255ce02b272"):
    """
    Workflow đồng bộ Bank Rate.

    Luồng xử lý:
      1. AUTH:     Lấy OAuth2 token từ Auth0 (dùng chung config với gold flow)
      2. PREFETCH: Gọi PowerBI API → lấy raw data (subquery tự động lọc ngày mới nhất)
      3. PARSE:    Parse response → list các row {bank, rate_1..rate_12}
      4. PUSH:     Push từng row lên bảng admin qua POST /v1/seo/tables/{id}

    Args:
        resource_key: x-powerbi-resourcekey header (có thể lấy từ Prefect Secret)
    """
    logger = get_run_logger()

    logger.info("🚀 Bắt đầu workflow Bank Rate Sync")
    logger.info(f"   API Base   : {CONFIG['API_BASE']}")
    logger.info(f"   Table ID   : {CONFIG['TABLE_ID']}")

    # ─── Bước 1: Auth ────────────────────────────────────────────────────
    token = get_token()

    # ─── Bước 2: Prefetch ───────────────────────────────────────────────
    raw_data = prefetch_bank_rates_raw(resource_key)

    # ─── Bước 3: Parse ──────────────────────────────────────────────────
    rows = parse_bank_rates(raw_data)

    # ─── Bước 4: Push từng row ──────────────────────────────────────────
    logger.info(f"💾 Push {len(rows)} rows lên bảng admin...")

    push_futures = []
    for row in rows:
        future = push_bank_rate.submit(token, CONFIG["TABLE_ID"], row)
        push_futures.append(future)

    # Chờ tất cả hoàn tất
    results = [f.result() for f in push_futures]

    # ─── Tổng kết ────────────────────────────────────────────────────────
    success_count = sum(1 for r in results if r["status"] == "ok")
    fail_count = sum(1 for r in results if r["status"] == "error")

    logger.info("═" * 50)
    logger.info(f"🏁 Hoàn thành!")
    logger.info(f"   ✅ Thành công: {success_count}")
    logger.info(f"   ❌ Thất bại  : {fail_count}")
    logger.info(f"   📋 Tổng cộng: {len(rows)}")
    logger.info("═" * 50)

    if fail_count > 0:
        raise Exception(f"Push thất bại {fail_count}/{len(rows)} rows")

    return {"success": success_count, "failed": fail_count, "total": len(rows)}


if __name__ == "__main__":
    bank_rate_sync_flow()
