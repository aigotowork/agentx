// Generated from actual emitted OpenAPI. Do not edit.
export const wireDocument = {
  "components": {
    "schemas": {
      "Answer": {
        "properties": {
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "message_id": {
            "type": "string"
          },
          "text": {
            "type": "string"
          }
        },
        "required": [
          "text",
          "message_id",
          "brief_revision"
        ],
        "type": "object"
      },
      "ApplyCall": {
        "additionalProperties": false,
        "properties": {
          "command": {
            "$ref": "#/components/schemas/OperatorCommand"
          },
          "command_id": {
            "type": "string"
          },
          "expected_revision": {
            "anyOf": [
              {
                "additionalProperties": false,
                "properties": {
                  "$integer": {
                    "pattern": "^(0|[1-9][0-9]*)$",
                    "type": "string"
                  }
                },
                "required": [
                  "$integer"
                ],
                "type": "object",
                "x-agentx-native-integer": "u-int64"
              },
              {
                "type": "null"
              }
            ],
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": [
                "integer",
                "null"
              ]
            }
          },
          "generation": {
            "type": "string"
          },
          "work": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/WorkRef"
              }
            ]
          }
        },
        "required": [
          "generation",
          "command_id",
          "command"
        ],
        "type": "object"
      },
      "Artifact": {
        "oneOf": [
          {
            "properties": {
              "Path": {
                "properties": {
                  "fingerprint": {
                    "type": "string"
                  },
                  "path": {
                    "type": "string"
                  }
                },
                "required": [
                  "path",
                  "fingerprint"
                ],
                "type": "object"
              }
            },
            "required": [
              "Path"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Git": {
                "properties": {
                  "path": {
                    "type": "string"
                  },
                  "revision": {
                    "type": "string"
                  }
                },
                "required": [
                  "revision",
                  "path"
                ],
                "type": "object"
              }
            },
            "required": [
              "Git"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Message": {
                "properties": {
                  "event": {
                    "additionalProperties": false,
                    "properties": {
                      "$integer": {
                        "pattern": "^(0|[1-9][0-9]*|-[1-9][0-9]*)$",
                        "type": "string"
                      }
                    },
                    "required": [
                      "$integer"
                    ],
                    "type": "object",
                    "x-agentx-native-integer": "int64",
                    "x-agentx-native-schema": {
                      "format": "int64",
                      "type": "integer"
                    }
                  }
                },
                "required": [
                  "event"
                ],
                "type": "object"
              }
            },
            "required": [
              "Message"
            ],
            "type": "object"
          },
          {
            "properties": {
              "External": {
                "properties": {
                  "reference": {
                    "type": "string"
                  }
                },
                "required": [
                  "reference"
                ],
                "type": "object"
              }
            },
            "required": [
              "External"
            ],
            "type": "object"
          }
        ]
      },
      "Assignment": {
        "properties": {
          "dispatched_brief": {
            "anyOf": [
              {
                "additionalProperties": false,
                "properties": {
                  "$integer": {
                    "pattern": "^(0|[1-9][0-9]*)$",
                    "type": "string"
                  }
                },
                "required": [
                  "$integer"
                ],
                "type": "object",
                "x-agentx-native-integer": "u-int64"
              },
              {
                "type": "null"
              }
            ],
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": [
                "integer",
                "null"
              ]
            }
          },
          "id": {
            "type": "string"
          },
          "participant": {
            "$ref": "#/components/schemas/ParticipantRef"
          },
          "role": {
            "$ref": "#/components/schemas/Role"
          }
        },
        "required": [
          "id",
          "role",
          "participant"
        ],
        "type": "object"
      },
      "Brief": {
        "properties": {
          "input": {
            "$ref": "#/components/schemas/BriefInput"
          },
          "revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "revision",
          "input"
        ],
        "type": "object"
      },
      "BriefInput": {
        "properties": {
          "criteria": {
            "items": {
              "$ref": "#/components/schemas/Criterion"
            },
            "type": "array"
          },
          "goal": {
            "type": "string"
          },
          "source": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/SourceBasis"
              }
            ]
          },
          "title": {
            "type": "string"
          }
        },
        "required": [
          "title",
          "goal",
          "criteria"
        ],
        "type": "object"
      },
      "Candidate": {
        "properties": {
          "label": {
            "type": "string"
          },
          "participant": {
            "$ref": "#/components/schemas/ParticipantRef"
          },
          "runtime_status": {
            "type": "string"
          },
          "tool": {
            "type": "string"
          },
          "unavailable": {
            "type": [
              "string",
              "null"
            ]
          }
        },
        "required": [
          "participant",
          "label",
          "tool",
          "runtime_status"
        ],
        "type": "object"
      },
      "Change": {
        "properties": {
          "command_id": {
            "type": "string"
          },
          "effect": {
            "$ref": "#/components/schemas/Effect"
          },
          "messages": {
            "items": {
              "$ref": "#/components/schemas/MessageReceipt"
            },
            "type": "array"
          },
          "reference": {
            "$ref": "#/components/schemas/WorkRef"
          },
          "replayed": {
            "type": "boolean"
          },
          "revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "command_id",
          "reference",
          "revision",
          "effect",
          "messages",
          "replayed"
        ],
        "type": "object"
      },
      "CheckInput": {
        "properties": {
          "criteria": {
            "items": {
              "$ref": "#/components/schemas/CriterionCheck"
            },
            "type": "array"
          },
          "evidence": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "observed_fingerprint": {
            "type": "string"
          },
          "request_id": {
            "type": "string"
          },
          "result_id": {
            "type": "string"
          },
          "verdict": {
            "$ref": "#/components/schemas/Verdict"
          }
        },
        "required": [
          "request_id",
          "result_id",
          "observed_fingerprint",
          "verdict",
          "criteria",
          "evidence"
        ],
        "type": "object"
      },
      "CheckReport": {
        "properties": {
          "assignment_id": {
            "type": "string"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "id": {
            "type": "string"
          },
          "input": {
            "$ref": "#/components/schemas/CheckInput"
          }
        },
        "required": [
          "id",
          "assignment_id",
          "brief_revision",
          "input",
          "at"
        ],
        "type": "object"
      },
      "CheckRequest": {
        "properties": {
          "assignment_id": {
            "type": "string"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "id": {
            "type": "string"
          },
          "message_id": {
            "type": "string"
          },
          "reason": {
            "type": "string"
          },
          "result_id": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "assignment_id",
          "result_id",
          "brief_revision",
          "reason",
          "message_id",
          "at"
        ],
        "type": "object"
      },
      "Criterion": {
        "properties": {
          "id": {
            "type": "string"
          },
          "required": {
            "type": "boolean"
          },
          "text": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "text",
          "required"
        ],
        "type": "object"
      },
      "CriterionCheck": {
        "properties": {
          "criterion_id": {
            "type": "string"
          },
          "evidence": {
            "type": "string"
          },
          "verdict": {
            "$ref": "#/components/schemas/Verdict"
          }
        },
        "required": [
          "criterion_id",
          "verdict",
          "evidence"
        ],
        "type": "object"
      },
      "Decision": {
        "properties": {
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "check_id": {
            "type": [
              "string",
              "null"
            ]
          },
          "id": {
            "type": "string"
          },
          "kind": {
            "$ref": "#/components/schemas/DecisionKind"
          },
          "message_id": {
            "type": [
              "string",
              "null"
            ]
          },
          "reason": {
            "type": "string"
          },
          "result_id": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "kind",
          "result_id",
          "brief_revision",
          "reason",
          "at"
        ],
        "type": "object"
      },
      "DecisionKind": {
        "enum": [
          "Accepted",
          "ChangesRequested"
        ],
        "type": "string"
      },
      "Descriptor": {
        "properties": {
          "available": {
            "type": "boolean"
          },
          "collaboration": {
            "type": "boolean"
          },
          "generation": {
            "type": [
              "string",
              "null"
            ]
          },
          "limits": {
            "$ref": "#/components/schemas/Limits"
          },
          "project": {
            "type": "string"
          },
          "project_label": {
            "type": "string"
          },
          "scope": {
            "type": "string"
          }
        },
        "required": [
          "scope",
          "project",
          "project_label",
          "collaboration",
          "available",
          "limits"
        ],
        "type": "object"
      },
      "Effect": {
        "enum": [
          "Saved",
          "MessageAccepted",
          "Reported",
          "Accepted"
        ],
        "type": "string"
      },
      "Eligibility": {
        "properties": {
          "closed": {
            "type": "boolean"
          },
          "reasons": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "selection": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ResultSelection"
              }
            ]
          }
        },
        "required": [
          "reasons",
          "closed"
        ],
        "type": "object"
      },
      "Empty": {
        "additionalProperties": false,
        "type": "object"
      },
      "Failure": {
        "oneOf": [
          {
            "properties": {
              "InvalidInput": {
                "properties": {
                  "field": {
                    "type": "string"
                  },
                  "reason": {
                    "type": "string"
                  }
                },
                "required": [
                  "field",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "InvalidInput"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Forbidden"
            ],
            "type": "string"
          },
          {
            "enum": [
              "StaleReference"
            ],
            "type": "string"
          },
          {
            "properties": {
              "RevisionConflict": {
                "properties": {
                  "current_revision": {
                    "additionalProperties": false,
                    "properties": {
                      "$integer": {
                        "pattern": "^(0|[1-9][0-9]*)$",
                        "type": "string"
                      }
                    },
                    "required": [
                      "$integer"
                    ],
                    "type": "object",
                    "x-agentx-native-integer": "u-int64",
                    "x-agentx-native-schema": {
                      "format": "u-int64",
                      "minimum": 0,
                      "type": "integer"
                    }
                  }
                },
                "required": [
                  "current_revision"
                ],
                "type": "object"
              }
            },
            "required": [
              "RevisionConflict"
            ],
            "type": "object"
          },
          {
            "enum": [
              "NotFound"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Capacity"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Ineligible": {
                "properties": {
                  "reasons": {
                    "items": {
                      "type": "string"
                    },
                    "type": "array"
                  }
                },
                "required": [
                  "reasons"
                ],
                "type": "object"
              }
            },
            "required": [
              "Ineligible"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Closed"
            ],
            "type": "string"
          },
          {
            "enum": [
              "CommandConflict"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Unavailable"
            ],
            "type": "string"
          },
          {
            "properties": {
              "OutcomeUnknown": {
                "properties": {
                  "command_id": {
                    "type": "string"
                  }
                },
                "required": [
                  "command_id"
                ],
                "type": "object"
              }
            },
            "required": [
              "OutcomeUnknown"
            ],
            "type": "object"
          }
        ]
      },
      "HttpError": {
        "properties": {
          "error": {
            "type": "string"
          }
        },
        "required": [
          "error"
        ],
        "type": "object"
      },
      "Limits": {
        "properties": {
          "artifacts": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "criteria": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "history": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "page": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "source_reference": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "source_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "text": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "title": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "text",
          "title",
          "source_reference",
          "source_revision",
          "criteria",
          "artifacts",
          "page",
          "history"
        ],
        "type": "object"
      },
      "MessageKind": {
        "enum": [
          "Assignment",
          "Note",
          "Answer",
          "CheckRequest",
          "Changes"
        ],
        "type": "string"
      },
      "MessageReceipt": {
        "properties": {
          "event": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*|-[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "int64",
            "x-agentx-native-schema": {
              "format": "int64",
              "type": "integer"
            }
          },
          "recipients": {
            "items": {
              "$ref": "#/components/schemas/ParticipantRef"
            },
            "type": "array"
          }
        },
        "required": [
          "event",
          "recipients"
        ],
        "type": "object"
      },
      "OperatorCommand": {
        "oneOf": [
          {
            "properties": {
              "CreateWork": {
                "properties": {
                  "brief": {
                    "$ref": "#/components/schemas/BriefInput"
                  },
                  "checker": {
                    "$ref": "#/components/schemas/ParticipantRef"
                  },
                  "implementer": {
                    "$ref": "#/components/schemas/ParticipantRef"
                  }
                },
                "required": [
                  "brief",
                  "implementer",
                  "checker"
                ],
                "type": "object"
              }
            },
            "required": [
              "CreateWork"
            ],
            "type": "object"
          },
          {
            "properties": {
              "UpdateBrief": {
                "$ref": "#/components/schemas/BriefInput"
              }
            },
            "required": [
              "UpdateBrief"
            ],
            "type": "object"
          },
          {
            "properties": {
              "BindParticipant": {
                "properties": {
                  "participant": {
                    "$ref": "#/components/schemas/ParticipantRef"
                  },
                  "role": {
                    "$ref": "#/components/schemas/Role"
                  }
                },
                "required": [
                  "role",
                  "participant"
                ],
                "type": "object"
              }
            },
            "required": [
              "BindParticipant"
            ],
            "type": "object"
          },
          {
            "properties": {
              "DispatchAssignment": {
                "properties": {
                  "role": {
                    "$ref": "#/components/schemas/Role"
                  }
                },
                "required": [
                  "role"
                ],
                "type": "object"
              }
            },
            "required": [
              "DispatchAssignment"
            ],
            "type": "object"
          },
          {
            "properties": {
              "SendNote": {
                "properties": {
                  "roles": {
                    "items": {
                      "$ref": "#/components/schemas/Role"
                    },
                    "type": "array"
                  },
                  "text": {
                    "type": "string"
                  }
                },
                "required": [
                  "roles",
                  "text"
                ],
                "type": "object"
              }
            },
            "required": [
              "SendNote"
            ],
            "type": "object"
          },
          {
            "properties": {
              "AnswerQuestion": {
                "properties": {
                  "amendment": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/BriefInput"
                      }
                    ]
                  },
                  "question_id": {
                    "type": "string"
                  },
                  "text": {
                    "type": "string"
                  }
                },
                "required": [
                  "question_id",
                  "text"
                ],
                "type": "object"
              }
            },
            "required": [
              "AnswerQuestion"
            ],
            "type": "object"
          },
          {
            "properties": {
              "RequestCheck": {
                "properties": {
                  "reason": {
                    "type": "string"
                  },
                  "result_id": {
                    "type": "string"
                  }
                },
                "required": [
                  "result_id",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "RequestCheck"
            ],
            "type": "object"
          },
          {
            "properties": {
              "AcceptResult": {
                "$ref": "#/components/schemas/ResultSelection"
              }
            },
            "required": [
              "AcceptResult"
            ],
            "type": "object"
          },
          {
            "properties": {
              "RequestChanges": {
                "properties": {
                  "check_id": {
                    "type": [
                      "string",
                      "null"
                    ]
                  },
                  "reason": {
                    "type": "string"
                  },
                  "result_id": {
                    "type": "string"
                  }
                },
                "required": [
                  "result_id",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "RequestChanges"
            ],
            "type": "object"
          }
        ]
      },
      "Page": {
        "properties": {
          "after": {
            "type": [
              "string",
              "null"
            ]
          },
          "limit": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "limit"
        ],
        "type": "object"
      },
      "ParticipantRef": {
        "description": "Canonical name plus exact existing `InstanceRow.created_at.to_bits()`.\nNames, OS PIDs and provider session ids alone are not row identity.",
        "properties": {
          "canonical_name": {
            "type": "string"
          },
          "incarnation_bits": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "canonical_name",
          "incarnation_bits"
        ],
        "type": "object"
      },
      "Progress": {
        "properties": {
          "assignment_id": {
            "type": "string"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "kind": {
            "$ref": "#/components/schemas/ProgressKind"
          },
          "summary": {
            "type": "string"
          }
        },
        "required": [
          "assignment_id",
          "brief_revision",
          "kind",
          "summary",
          "at"
        ],
        "type": "object"
      },
      "ProgressKind": {
        "enum": [
          "Acknowledged",
          "Started",
          "Progress"
        ],
        "type": "string"
      },
      "Question": {
        "properties": {
          "answer": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/Answer"
              }
            ]
          },
          "assignment_id": {
            "type": "string"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "id": {
            "type": "string"
          },
          "prompt": {
            "type": "string"
          },
          "suggestions": {
            "items": {
              "type": "string"
            },
            "type": "array"
          }
        },
        "required": [
          "id",
          "assignment_id",
          "brief_revision",
          "prompt",
          "suggestions",
          "at"
        ],
        "type": "object"
      },
      "Read": {
        "oneOf": [
          {
            "properties": {
              "List": {
                "$ref": "#/components/schemas/Page"
              }
            },
            "required": [
              "List"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Detail": {
                "$ref": "#/components/schemas/WorkRef"
              }
            },
            "required": [
              "Detail"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Candidates": {
                "$ref": "#/components/schemas/Page"
              }
            },
            "required": [
              "Candidates"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Command": {
                "properties": {
                  "command_id": {
                    "type": "string"
                  }
                },
                "required": [
                  "command_id"
                ],
                "type": "object"
              }
            },
            "required": [
              "Command"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Thread": {
                "properties": {
                  "page": {
                    "$ref": "#/components/schemas/ThreadPage"
                  },
                  "work": {
                    "$ref": "#/components/schemas/WorkRef"
                  }
                },
                "required": [
                  "work",
                  "page"
                ],
                "type": "object"
              }
            },
            "required": [
              "Thread"
            ],
            "type": "object"
          }
        ]
      },
      "ReadCall": {
        "additionalProperties": false,
        "properties": {
          "generation": {
            "type": "string"
          },
          "read": {
            "$ref": "#/components/schemas/Read"
          }
        },
        "required": [
          "generation",
          "read"
        ],
        "type": "object"
      },
      "ReadValue": {
        "oneOf": [
          {
            "properties": {
              "List": {
                "properties": {
                  "items": {
                    "items": {
                      "$ref": "#/components/schemas/WorkSummary"
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
                  "items"
                ],
                "type": "object"
              }
            },
            "required": [
              "List"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Detail": {
                "$ref": "#/components/schemas/WorkDetail"
              }
            },
            "required": [
              "Detail"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Candidates": {
                "properties": {
                  "items": {
                    "items": {
                      "$ref": "#/components/schemas/Candidate"
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
                  "items"
                ],
                "type": "object"
              }
            },
            "required": [
              "Candidates"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Command": {
                "oneOf": [
                  {
                    "type": "null"
                  },
                  {
                    "$ref": "#/components/schemas/Change"
                  }
                ]
              }
            },
            "required": [
              "Command"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Thread": {
                "properties": {
                  "items": {
                    "items": {
                      "$ref": "#/components/schemas/ThreadMessage"
                    },
                    "type": "array"
                  },
                  "next": {
                    "anyOf": [
                      {
                        "additionalProperties": false,
                        "properties": {
                          "$integer": {
                            "pattern": "^(0|[1-9][0-9]*|-[1-9][0-9]*)$",
                            "type": "string"
                          }
                        },
                        "required": [
                          "$integer"
                        ],
                        "type": "object",
                        "x-agentx-native-integer": "int64"
                      },
                      {
                        "type": "null"
                      }
                    ],
                    "x-agentx-native-schema": {
                      "format": "int64",
                      "type": [
                        "integer",
                        "null"
                      ]
                    }
                  }
                },
                "required": [
                  "items"
                ],
                "type": "object"
              }
            },
            "required": [
              "Thread"
            ],
            "type": "object"
          }
        ]
      },
      "Response": {
        "oneOf": [
          {
            "properties": {
              "Read": {
                "$ref": "#/components/schemas/ReadValue"
              }
            },
            "required": [
              "Read"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Applied": {
                "$ref": "#/components/schemas/Change"
              }
            },
            "required": [
              "Applied"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Rejected": {
                "$ref": "#/components/schemas/Failure"
              }
            },
            "required": [
              "Rejected"
            ],
            "type": "object"
          }
        ]
      },
      "ResultInput": {
        "properties": {
          "artifacts": {
            "items": {
              "$ref": "#/components/schemas/Artifact"
            },
            "type": "array"
          },
          "source": {
            "$ref": "#/components/schemas/SourceBasis"
          },
          "summary": {
            "type": "string"
          }
        },
        "required": [
          "summary",
          "source",
          "artifacts"
        ],
        "type": "object"
      },
      "ResultSelection": {
        "properties": {
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "check_id": {
            "type": "string"
          },
          "result_id": {
            "type": "string"
          }
        },
        "required": [
          "result_id",
          "check_id",
          "brief_revision"
        ],
        "type": "object"
      },
      "Role": {
        "enum": [
          "Implementer",
          "Checker"
        ],
        "type": "string"
      },
      "SourceBasis": {
        "properties": {
          "reference": {
            "type": "string"
          },
          "revision": {
            "type": "string"
          }
        },
        "required": [
          "reference",
          "revision"
        ],
        "type": "object"
      },
      "ThreadMessage": {
        "properties": {
          "at": {
            "type": [
              "string",
              "null"
            ]
          },
          "author": {
            "type": "string"
          },
          "event": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*|-[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "int64",
            "x-agentx-native-schema": {
              "format": "int64",
              "type": "integer"
            }
          },
          "intent": {
            "type": [
              "string",
              "null"
            ]
          },
          "reply_to": {
            "type": [
              "string",
              "null"
            ]
          },
          "text": {
            "type": "string"
          }
        },
        "required": [
          "event",
          "author",
          "text"
        ],
        "type": "object"
      },
      "ThreadPage": {
        "properties": {
          "after": {
            "anyOf": [
              {
                "additionalProperties": false,
                "properties": {
                  "$integer": {
                    "pattern": "^(0|[1-9][0-9]*|-[1-9][0-9]*)$",
                    "type": "string"
                  }
                },
                "required": [
                  "$integer"
                ],
                "type": "object",
                "x-agentx-native-integer": "int64"
              },
              {
                "type": "null"
              }
            ],
            "x-agentx-native-schema": {
              "format": "int64",
              "type": [
                "integer",
                "null"
              ]
            }
          },
          "limit": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "limit"
        ],
        "type": "object"
      },
      "Verdict": {
        "enum": [
          "Passed",
          "Failed",
          "Unable"
        ],
        "type": "string"
      },
      "Work": {
        "properties": {
          "assignments": {
            "items": {
              "$ref": "#/components/schemas/Assignment"
            },
            "type": "array"
          },
          "briefs": {
            "items": {
              "$ref": "#/components/schemas/Brief"
            },
            "type": "array"
          },
          "check_requests": {
            "items": {
              "$ref": "#/components/schemas/CheckRequest"
            },
            "type": "array"
          },
          "checker": {
            "type": "string"
          },
          "checks": {
            "items": {
              "$ref": "#/components/schemas/CheckReport"
            },
            "type": "array"
          },
          "created_at": {
            "type": "string"
          },
          "decisions": {
            "items": {
              "$ref": "#/components/schemas/Decision"
            },
            "type": "array"
          },
          "implementer": {
            "type": "string"
          },
          "messages": {
            "items": {
              "$ref": "#/components/schemas/WorkMessage"
            },
            "type": "array"
          },
          "progress": {
            "items": {
              "$ref": "#/components/schemas/Progress"
            },
            "type": "array"
          },
          "questions": {
            "items": {
              "$ref": "#/components/schemas/Question"
            },
            "type": "array"
          },
          "reference": {
            "$ref": "#/components/schemas/WorkRef"
          },
          "results": {
            "items": {
              "$ref": "#/components/schemas/WorkResult"
            },
            "type": "array"
          },
          "revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "reference",
          "revision",
          "briefs",
          "assignments",
          "implementer",
          "checker",
          "messages",
          "progress",
          "questions",
          "results",
          "check_requests",
          "checks",
          "decisions",
          "created_at"
        ],
        "type": "object"
      },
      "WorkDetail": {
        "properties": {
          "eligibility": {
            "$ref": "#/components/schemas/Eligibility"
          },
          "work": {
            "$ref": "#/components/schemas/Work"
          }
        },
        "required": [
          "work",
          "eligibility"
        ],
        "type": "object"
      },
      "WorkMessage": {
        "properties": {
          "assignment_ids": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "id": {
            "type": "string"
          },
          "kind": {
            "$ref": "#/components/schemas/MessageKind"
          },
          "receipt": {
            "$ref": "#/components/schemas/MessageReceipt"
          },
          "result_id": {
            "type": [
              "string",
              "null"
            ]
          },
          "text": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "kind",
          "text",
          "receipt",
          "at",
          "assignment_ids",
          "brief_revision"
        ],
        "type": "object"
      },
      "WorkRef": {
        "properties": {
          "generation": {
            "type": "string"
          },
          "id": {
            "type": "string"
          },
          "project": {
            "type": "string"
          },
          "scope": {
            "type": "string"
          }
        },
        "required": [
          "scope",
          "generation",
          "project",
          "id"
        ],
        "type": "object"
      },
      "WorkResult": {
        "properties": {
          "assignment_id": {
            "type": "string"
          },
          "at": {
            "type": "string"
          },
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "id": {
            "type": "string"
          },
          "input": {
            "$ref": "#/components/schemas/ResultInput"
          }
        },
        "required": [
          "id",
          "assignment_id",
          "brief_revision",
          "input",
          "at"
        ],
        "type": "object"
      },
      "WorkSummary": {
        "properties": {
          "brief_revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "checker": {
            "$ref": "#/components/schemas/ParticipantRef"
          },
          "closed": {
            "type": "boolean"
          },
          "current_result": {
            "type": [
              "string",
              "null"
            ]
          },
          "implementer": {
            "$ref": "#/components/schemas/ParticipantRef"
          },
          "open_questions": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int32",
            "x-agentx-native-schema": {
              "format": "u-int32",
              "minimum": 0,
              "type": "integer"
            }
          },
          "reference": {
            "$ref": "#/components/schemas/WorkRef"
          },
          "revision": {
            "additionalProperties": false,
            "properties": {
              "$integer": {
                "pattern": "^(0|[1-9][0-9]*)$",
                "type": "string"
              }
            },
            "required": [
              "$integer"
            ],
            "type": "object",
            "x-agentx-native-integer": "u-int64",
            "x-agentx-native-schema": {
              "format": "u-int64",
              "minimum": 0,
              "type": "integer"
            }
          },
          "title": {
            "type": "string"
          }
        },
        "required": [
          "reference",
          "revision",
          "title",
          "brief_revision",
          "implementer",
          "checker",
          "open_questions",
          "closed"
        ],
        "type": "object"
      }
    }
  },
  "info": {
    "contact": {
      "name": "agentx"
    },
    "description": "Fixed-scope/project workbench. Every integer uses the native $integer marker codec. Exact numeric local Host and same Origin admission; POST application/json only. Collaboration writes require listener collaboration=true. Input exposes only OperatorCommand, never AgentReport or caller actor context. A 200 Rejected response is a domain refusal, distinct from transport errors. Original command_id/generation and payload recover durable native changes; do not silently rebase revisions or replace identities.",
    "license": {
      "identifier": "PolyForm-Noncommercial-1.0.0",
      "name": "PolyForm-Noncommercial-1.0.0"
    },
    "title": "AgentX Workbench",
    "version": "1"
  },
  "openapi": "3.1.0",
  "paths": {
    "/api/v1/apply": {
      "post": {
        "operationId": "workbenchApply",
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/ApplyCall"
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
                  "$ref": "#/components/schemas/Response"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "403": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "405": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "408": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "413": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "415": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          }
        },
        "summary": "Execute only an operator command; a 200 Rejected is not transport failure.",
        "tags": [
          "super"
        ]
      }
    },
    "/api/v1/connection": {
      "post": {
        "operationId": "workbenchConnection",
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/Empty"
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
                  "$ref": "#/components/schemas/Descriptor"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "403": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "405": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "408": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "413": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "415": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          }
        },
        "summary": "Describe this fixed listener; unavailable generation is retained as null.",
        "tags": [
          "super"
        ]
      }
    },
    "/api/v1/read": {
      "post": {
        "operationId": "workbenchRead",
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/ReadCall"
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
                  "$ref": "#/components/schemas/Response"
                }
              }
            },
            "description": ""
          },
          "400": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "403": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "405": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "408": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "413": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "415": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          },
          "503": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/HttpError"
                }
              }
            },
            "description": ""
          }
        },
        "summary": "Execute the closed five-read native boundary in the fixed project.",
        "tags": [
          "super"
        ]
      }
    }
  },
  "x-agentx-codec": {
    "decode": "Every native integer is a single-key marker object containing canonical decimal text; convert schema-declared markers to bigint with native width/sign bounds. Identity strings remain strings.",
    "encode": "Emit native bigint operands as single-key markers; bare numeric tokens, extra keys, -0 and leading zeroes are rejected. Existing strict JSON/native validation remains authoritative.",
    "integerMarker": "$integer",
    "nativeIntegerMetadata": "x-agentx-native-integer",
    "wire": "integer-marker-v1"
  }
} as const;
