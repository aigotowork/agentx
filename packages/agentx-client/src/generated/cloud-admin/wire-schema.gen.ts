// Generated from actual emitted OpenAPI. Do not edit.
export const wireDocument = {
  "components": {
    "schemas": {
      "AdminOverview": {
        "properties": {
          "agents": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "deployment_id": {
            "type": "string"
          },
          "hosts": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "namespaces": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "observed_at": {
            "format": "int64",
            "type": "integer"
          },
          "online_hosts": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "protocol_version": {
            "description": "Diagnostic version; operation contracts determine compatibility.",
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "unavailable": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          }
        },
        "required": [
          "deployment_id",
          "protocol_version",
          "observed_at",
          "namespaces",
          "hosts",
          "online_hosts",
          "agents",
          "unavailable"
        ],
        "type": "object"
      },
      "AgentPage": {
        "properties": {
          "generation": {
            "description": "Namespace generation verified in the same transaction as the page.",
            "type": "string"
          },
          "items": {
            "items": {
              "$ref": "#/components/schemas/CloudAgentSummary"
            },
            "type": "array"
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "generation",
          "items",
          "next"
        ],
        "type": "object"
      },
      "AgentView": {
        "additionalProperties": false,
        "properties": {
          "host": {
            "type": "string"
          },
          "incarnation": {
            "pattern": "^(?:[A-Za-z0-9]{12}|~human)$",
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "workspace": {
            "$ref": "#/components/schemas/AgentWorkspace"
          }
        },
        "required": [
          "name",
          "host",
          "incarnation",
          "workspace"
        ],
        "type": "object"
      },
      "AgentWorkspace": {
        "description": "Membership chosen when the exact Agent incarnation is prepared for launch.",
        "oneOf": [
          {
            "properties": {
              "kind": {
                "enum": [
                  "project"
                ],
                "type": "string"
              },
              "workspace_id": {
                "pattern": "^WS[A-Za-z0-9]{10}$",
                "type": "string"
              }
            },
            "required": [
              "workspace_id",
              "kind"
            ],
            "type": "object"
          },
          {
            "properties": {
              "kind": {
                "enum": [
                  "global"
                ],
                "type": "string"
              }
            },
            "required": [
              "kind"
            ],
            "type": "object"
          },
          {
            "properties": {
              "kind": {
                "enum": [
                  "unexposed"
                ],
                "type": "string"
              }
            },
            "required": [
              "kind"
            ],
            "type": "object"
          }
        ]
      },
      "ApiError": {
        "properties": {
          "code": {
            "type": "string"
          },
          "message": {
            "type": "string"
          }
        },
        "required": [
          "code",
          "message"
        ],
        "type": "object"
      },
      "CloudAgentFacts": {
        "additionalProperties": false,
        "properties": {
          "cwd": {
            "type": "string"
          },
          "role": {
            "type": [
              "string",
              "null"
            ]
          },
          "status": {
            "$ref": "#/components/schemas/CloudAgentStatus"
          },
          "title": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "role",
          "title",
          "cwd",
          "status"
        ],
        "type": "object"
      },
      "CloudAgentStatus": {
        "enum": [
          "active",
          "idle",
          "error",
          "unknown",
          "stopped"
        ],
        "type": "string"
      },
      "CloudAgentSummary": {
        "properties": {
          "facts": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/CloudAgentFacts"
              }
            ]
          },
          "host": {
            "type": "string"
          },
          "incarnation": {
            "pattern": "^(?:[A-Za-z0-9]{12})$",
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "workspace": {
            "$ref": "#/components/schemas/AgentWorkspace"
          }
        },
        "required": [
          "name",
          "host",
          "incarnation",
          "workspace"
        ],
        "type": "object"
      },
      "CloudTranscriptRecord": {
        "properties": {
          "id": {
            "type": "string"
          },
          "role": {
            "$ref": "#/components/schemas/CloudTranscriptRole"
          },
          "text": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "role",
          "text"
        ],
        "type": "object"
      },
      "CloudTranscriptRole": {
        "enum": [
          "user",
          "assistant"
        ],
        "type": "string"
      },
      "ControlCommand": {
        "oneOf": [
          {
            "properties": {
              "create_directory": {
                "description": "Create the final component if absent. Parents must already exist.",
                "type": [
                  "boolean",
                  "null"
                ]
              },
              "initial_prompt": {
                "maxLength": 16384,
                "type": [
                  "string",
                  "null"
                ]
              },
              "kind": {
                "enum": [
                  "launch"
                ],
                "type": "string"
              },
              "name": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "role": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "title": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "working_directory": {
                "description": "Slash-separated path relative to the workspace export. Omit or send an empty string for its root.",
                "maxLength": 1024,
                "type": [
                  "string",
                  "null"
                ]
              },
              "workspace_id": {
                "pattern": "^WS[A-Za-z0-9]{10}$",
                "type": "string"
              }
            },
            "required": [
              "workspace_id",
              "kind"
            ],
            "type": "object"
          },
          {
            "properties": {
              "incarnation": {
                "pattern": "^[A-Za-z0-9]{12}$",
                "type": "string"
              },
              "kind": {
                "enum": [
                  "stop"
                ],
                "type": "string"
              },
              "name": {
                "type": "string"
              }
            },
            "required": [
              "name",
              "incarnation",
              "kind"
            ],
            "type": "object"
          },
          {
            "properties": {
              "create_directory": {
                "description": "Create only the final component if absent. Parents must already exist.",
                "type": "boolean"
              },
              "directory": {
                "description": "Nonempty slash-separated path relative to the parent export.",
                "maxLength": 1024,
                "minLength": 1,
                "type": "string"
              },
              "kind": {
                "enum": [
                  "add_workspace"
                ],
                "type": "string"
              },
              "name": {
                "description": "Display name to publish for this project.",
                "type": "string"
              },
              "parent": {
                "description": "Alias of an eligible configured export.",
                "type": "string"
              }
            },
            "required": [
              "parent",
              "directory",
              "name",
              "create_directory",
              "kind"
            ],
            "type": "object"
          }
        ]
      },
      "ControlOutcome": {
        "oneOf": [
          {
            "properties": {
              "agent": {
                "$ref": "#/components/schemas/AgentView"
              },
              "status": {
                "enum": [
                  "succeeded"
                ],
                "type": "string"
              }
            },
            "required": [
              "agent",
              "status"
            ],
            "type": "object"
          },
          {
            "properties": {
              "status": {
                "enum": [
                  "workspace_added"
                ],
                "type": "string"
              },
              "workspace_id": {
                "pattern": "^WS[A-Za-z0-9]{10}$",
                "type": "string"
              }
            },
            "required": [
              "workspace_id",
              "status"
            ],
            "type": "object"
          },
          {
            "properties": {
              "code": {
                "type": "string"
              },
              "status": {
                "enum": [
                  "failed"
                ],
                "type": "string"
              }
            },
            "required": [
              "code",
              "status"
            ],
            "type": "object"
          },
          {
            "properties": {
              "code": {
                "type": "string"
              },
              "status": {
                "enum": [
                  "uncertain"
                ],
                "type": "string"
              }
            },
            "required": [
              "code",
              "status"
            ],
            "type": "object"
          }
        ]
      },
      "ControlReceipt": {
        "properties": {
          "claim_id": {
            "type": [
              "string",
              "null"
            ]
          },
          "command": {
            "$ref": "#/components/schemas/ControlCommand"
          },
          "generation": {
            "type": "string"
          },
          "host": {
            "type": "string"
          },
          "installation_id": {
            "type": "string"
          },
          "outcome": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ControlOutcome"
              }
            ]
          },
          "request_id": {
            "type": "string"
          },
          "status": {
            "$ref": "#/components/schemas/ControlStatus"
          }
        },
        "required": [
          "generation",
          "request_id",
          "host",
          "installation_id",
          "command",
          "status",
          "claim_id",
          "outcome"
        ],
        "type": "object"
      },
      "ControlStatus": {
        "enum": [
          "queued",
          "running",
          "succeeded",
          "failed",
          "uncertain"
        ],
        "type": "string"
      },
      "CreateControlOperation": {
        "additionalProperties": false,
        "properties": {
          "command": {
            "$ref": "#/components/schemas/ControlCommand"
          },
          "generation": {
            "type": "string"
          },
          "host": {
            "type": "string"
          },
          "installation_id": {
            "type": "string"
          },
          "request_id": {
            "type": "string"
          }
        },
        "required": [
          "generation",
          "request_id",
          "host",
          "installation_id",
          "command"
        ],
        "type": "object"
      },
      "CreateNamespace": {
        "additionalProperties": false,
        "properties": {
          "request_id": {
            "maxLength": 128,
            "minLength": 1,
            "type": "string"
          },
          "token": {
            "maxLength": 12,
            "minLength": 12,
            "pattern": "^[A-Za-z0-9]{12}$",
            "type": "string",
            "writeOnly": true
          }
        },
        "required": [
          "request_id",
          "token"
        ],
        "type": "object"
      },
      "HostCapabilities": {
        "properties": {
          "directory_launch": {
            "description": "This control lease accepts workspace-relative launch directories.",
            "type": [
              "boolean",
              "null"
            ]
          },
          "generation": {
            "type": "string"
          },
          "host": {
            "type": "string"
          },
          "installation_id": {
            "type": "string"
          },
          "observed_at": {
            "format": "int64",
            "type": [
              "integer",
              "null"
            ]
          },
          "online": {
            "type": "boolean"
          },
          "project_roots": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "workspace_management": {
            "description": "This active control lease accepts remote workspace registration.",
            "type": [
              "boolean",
              "null"
            ]
          },
          "workspaces": {
            "items": {
              "$ref": "#/components/schemas/WorkspaceCapability"
            },
            "type": "array"
          }
        },
        "required": [
          "generation",
          "host",
          "installation_id",
          "online",
          "observed_at",
          "workspaces",
          "project_roots"
        ],
        "type": "object"
      },
      "HostPage": {
        "properties": {
          "generation": {
            "description": "Namespace generation verified in the same transaction as the page.",
            "type": "string"
          },
          "items": {
            "items": {
              "$ref": "#/components/schemas/HostView"
            },
            "type": "array"
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "generation",
          "items",
          "next"
        ],
        "type": "object"
      },
      "HostView": {
        "properties": {
          "last_seen": {
            "format": "int64",
            "type": "integer"
          },
          "name": {
            "type": "string"
          },
          "online": {
            "type": "boolean"
          }
        },
        "required": [
          "name",
          "last_seen",
          "online"
        ],
        "type": "object"
      },
      "Intent": {
        "enum": [
          "request",
          "inform",
          "ack"
        ],
        "type": "string"
      },
      "MessageHistoryPage": {
        "description": "Durable Namespace traffic, in descending message ID order. This is not a Provider transcript.",
        "properties": {
          "items": {
            "items": {
              "$ref": "#/components/schemas/MessageView"
            },
            "type": "array"
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "items",
          "next"
        ],
        "type": "object"
      },
      "MessageView": {
        "properties": {
          "body": {
            "type": "string"
          },
          "created_at": {
            "format": "int64",
            "type": "integer"
          },
          "from": {
            "$ref": "#/components/schemas/AgentView"
          },
          "id": {
            "type": "string"
          },
          "intent": {
            "$ref": "#/components/schemas/Intent"
          },
          "reply_to": {
            "type": [
              "string",
              "null"
            ]
          },
          "thread": {
            "type": [
              "string",
              "null"
            ]
          },
          "to": {
            "$ref": "#/components/schemas/AgentView"
          }
        },
        "required": [
          "id",
          "from",
          "to",
          "body",
          "intent",
          "reply_to",
          "thread",
          "created_at"
        ],
        "type": "object"
      },
      "NamespacePage": {
        "properties": {
          "items": {
            "items": {
              "$ref": "#/components/schemas/NamespaceRow"
            },
            "type": "array"
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "items",
          "next"
        ],
        "type": "object"
      },
      "NamespaceReceipt": {
        "properties": {
          "credential_version": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "generation": {
            "type": "string"
          },
          "namespace_id": {
            "type": "string"
          },
          "namespace_url": {
            "type": "string"
          }
        },
        "required": [
          "namespace_id",
          "namespace_url",
          "credential_version",
          "generation"
        ],
        "type": "object"
      },
      "NamespaceRow": {
        "properties": {
          "agents": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "available": {
            "type": "boolean"
          },
          "created_at": {
            "format": "int64",
            "type": "integer"
          },
          "credential_version": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "generation": {
            "type": "string"
          },
          "hosts": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "namespace_id": {
            "type": "string"
          },
          "namespace_url": {
            "type": "string"
          },
          "online_hosts": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          }
        },
        "required": [
          "namespace_id",
          "namespace_url",
          "credential_version",
          "generation",
          "created_at",
          "hosts",
          "online_hosts",
          "agents",
          "available"
        ],
        "type": "object"
      },
      "RoleCapability": {
        "additionalProperties": false,
        "properties": {
          "available": {
            "type": "boolean"
          },
          "name": {
            "type": "string"
          },
          "reason": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "name",
          "available",
          "reason"
        ],
        "type": "object"
      },
      "RotateToken": {
        "additionalProperties": false,
        "properties": {
          "expected_version": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "request_id": {
            "maxLength": 128,
            "minLength": 1,
            "type": "string"
          },
          "token": {
            "maxLength": 12,
            "minLength": 12,
            "pattern": "^[A-Za-z0-9]{12}$",
            "type": "string",
            "writeOnly": true
          }
        },
        "required": [
          "request_id",
          "expected_version",
          "token"
        ],
        "type": "object"
      },
      "SendHumanMessage": {
        "additionalProperties": false,
        "description": "Submit an operator message to one exact Agent registration.",
        "properties": {
          "body": {
            "description": "Nonblank message text, at most 64 KiB of UTF-8.",
            "type": "string"
          },
          "generation": {
            "type": "string"
          },
          "request_id": {
            "maxLength": 128,
            "minLength": 1,
            "type": "string"
          },
          "to": {
            "$ref": "#/components/schemas/AgentView"
          }
        },
        "required": [
          "generation",
          "request_id",
          "to",
          "body"
        ],
        "type": "object"
      },
      "SpaceChanges": {
        "properties": {
          "epoch": {
            "type": "string"
          },
          "messages": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "roster": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          },
          "transcript": {
            "format": "u-int32",
            "minimum": 0,
            "type": "integer"
          }
        },
        "required": [
          "epoch",
          "roster",
          "messages",
          "transcript"
        ],
        "type": "object"
      },
      "TranscriptHistoryPage": {
        "properties": {
          "agent": {
            "$ref": "#/components/schemas/AgentView"
          },
          "available": {
            "type": "boolean"
          },
          "facts": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/CloudAgentFacts"
              }
            ]
          },
          "items": {
            "items": {
              "$ref": "#/components/schemas/CloudTranscriptRecord"
            },
            "type": "array"
          },
          "latest": {
            "type": [
              "string",
              "null"
            ]
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "agent",
          "available",
          "items",
          "next",
          "latest",
          "facts"
        ],
        "type": "object"
      },
      "WorkspaceCapability": {
        "description": "Shared fields also composed into the flattened WorkspaceView schema.",
        "properties": {
          "default_available": {
            "type": "boolean"
          },
          "default_reason": {
            "type": [
              "string",
              "null"
            ]
          },
          "name": {
            "type": "string"
          },
          "roles": {
            "items": {
              "$ref": "#/components/schemas/RoleCapability"
            },
            "type": "array"
          },
          "workspace_id": {
            "pattern": "^WS[A-Za-z0-9]{10}$",
            "type": "string"
          }
        },
        "required": [
          "workspace_id",
          "name",
          "default_available",
          "default_reason",
          "roles"
        ],
        "type": "object"
      },
      "WorkspacePage": {
        "properties": {
          "generation": {
            "description": "Namespace generation verified in the same transaction as the page.",
            "type": "string"
          },
          "items": {
            "items": {
              "$ref": "#/components/schemas/WorkspaceView"
            },
            "type": "array"
          },
          "next": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "generation",
          "items",
          "next"
        ],
        "type": "object"
      },
      "WorkspaceView": {
        "allOf": [
          {
            "$ref": "#/components/schemas/WorkspaceCapability"
          },
          {
            "properties": {
              "host": {
                "type": "string"
              },
              "online": {
                "type": "boolean"
              }
            },
            "required": [
              "host",
              "online"
            ],
            "type": "object"
          }
        ]
      }
    },
    "securitySchemes": {
      "adminBearer": {
        "scheme": "bearer",
        "type": "http"
      }
    }
  },
  "info": {
    "contact": {
      "name": "agentx"
    },
    "description": "Connect Claude Code, Gemini CLI, Codex, OpenCode, Kilo Code, Pi, Oh My Pi, Antigravity, Cursor, Kimi, Copilot, Qoder CLI, and Grok Build so agents can message, watch, and spawn each other across terminals",
    "license": {
      "identifier": "PolyForm-Noncommercial-1.0.0",
      "name": "PolyForm-Noncommercial-1.0.0"
    },
    "title": "AgentX Deployment Administration",
    "version": "0.7.36"
  },
  "openapi": "3.1.0",
  "paths": {
    "/api/v1/admin/namespaces": {
      "get": {
        "operationId": "listAdminNamespaces",
        "parameters": [
          {
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/NamespacePage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}": {
      "get": {
        "operationId": "getAdminNamespace",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/NamespaceRow"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      },
      "put": {
        "operationId": "putAdminNamespace",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/CreateNamespace"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/NamespaceReceipt"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/agents": {
      "get": {
        "operationId": "listAdminNamespaceAgents",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Expected Namespace generation. Bind the first page's generation when continuing a cursor.",
            "in": "query",
            "name": "generation",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "maximum": 200,
              "minimum": 1,
              "type": "integer"
            }
          },
          {
            "in": "query",
            "name": "host",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "workspace_id",
            "required": false,
            "schema": {
              "pattern": "^WS[A-Za-z0-9]{10}$",
              "type": "string"
            }
          },
          {
            "description": "Include host-reported Agent facts. Omit to return identity and membership only.",
            "in": "query",
            "name": "facts",
            "required": false,
            "schema": {
              "type": "boolean"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/AgentPage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/changes": {
      "get": {
        "operationId": "adminSpaceChanges",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "generation",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "cursor",
            "required": false,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/SpaceChanges"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/hosts": {
      "get": {
        "operationId": "listAdminNamespaceHosts",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Expected Namespace generation. Bind the first page's generation when continuing a cursor.",
            "in": "query",
            "name": "generation",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "maximum": 200,
              "minimum": 1,
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HostPage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/hosts/{host}/capabilities": {
      "get": {
        "operationId": "getAdminHostCapabilities",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "path",
            "name": "host",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HostCapabilities"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/messages": {
      "get": {
        "operationId": "listAdminNamespaceMessages",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "generation",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "host",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "name",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "incarnation",
            "required": true,
            "schema": {
              "pattern": "^(?:[A-Za-z0-9]{12})$",
              "type": "string"
            }
          },
          {
            "description": "Exclusive forward cursor. Zero starts at the beginning. Forward pages use ascending IDs.",
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Exclusive positive decimal message ID; omit to start with the latest messages.",
            "in": "query",
            "name": "before",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "maximum": 100,
              "minimum": 1,
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/MessageHistoryPage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      },
      "post": {
        "operationId": "sendAdminNamespaceMessage",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/SendHumanMessage"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/MessageView"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "413": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Payload too large"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/operations": {
      "post": {
        "operationId": "createAdminControlOperation",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/CreateControlOperation"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ControlReceipt"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/operations/{request_id}": {
      "get": {
        "operationId": "getAdminControlOperation",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "path",
            "name": "request_id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ControlReceipt"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/token-rotations": {
      "post": {
        "operationId": "rotateAdminNamespaceToken",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/RotateToken"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/NamespaceReceipt"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/transcript": {
      "get": {
        "operationId": "listAdminNamespaceTranscript",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "generation",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "host",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "name",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "incarnation",
            "required": true,
            "schema": {
              "pattern": "^(?:[A-Za-z0-9]{12})$",
              "type": "string"
            }
          },
          {
            "description": "Exclusive positive decimal cursor for older records, newest first.",
            "in": "query",
            "name": "before",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Exclusive decimal cursor for newer records, oldest first. Use 0 to follow an empty transcript.",
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "maximum": 50,
              "minimum": 1,
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/TranscriptHistoryPage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/namespaces/{id}/workspaces": {
      "get": {
        "operationId": "listAdminWorkspaces",
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Expected Namespace generation. Bind the first page's generation when continuing a cursor.",
            "in": "query",
            "name": "generation",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "host",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "description": "Exclusive cursor from a previous page.",
            "in": "query",
            "name": "after",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "limit",
            "required": false,
            "schema": {
              "format": "u-int32",
              "maximum": 100,
              "minimum": 1,
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/WorkspacePage"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    },
    "/api/v1/admin/overview": {
      "get": {
        "operationId": "getAdminOverview",
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/AdminOverview"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid input"
          },
          "401": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Invalid admin credential"
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unknown namespace"
          },
          "409": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Conflict"
          },
          "500": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Storage error"
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ApiError"
                }
              }
            },
            "description": "Unavailable"
          }
        },
        "tags": []
      }
    }
  },
  "security": [
    {
      "adminBearer": []
    }
  ]
} as const;
