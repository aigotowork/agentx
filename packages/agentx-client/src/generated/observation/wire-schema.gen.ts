// Generated from actual emitted OpenAPI. Do not edit.
export const wireDocument = {
  "components": {
    "schemas": {
      "AcceptedPeer": {
        "properties": {
          "connection": {
            "$ref": "#/components/schemas/ConnectionRef"
          },
          "observed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "process": {
            "$ref": "#/components/schemas/ObservedProcess"
          }
        },
        "required": [
          "connection",
          "process",
          "observed_at"
        ],
        "type": "object"
      },
      "ActivationTarget": {
        "description": "Checked both when scheduling and immediately before a provider effect.",
        "properties": {
          "binding": {
            "$ref": "#/components/schemas/BindingId"
          },
          "execution": {
            "$ref": "#/components/schemas/ExecutionId"
          },
          "participant": {
            "$ref": "#/components/schemas/ParticipantRef"
          }
        },
        "required": [
          "participant",
          "execution",
          "binding"
        ],
        "type": "object"
      },
      "ApplicationResultClass": {
        "oneOf": [
          {
            "enum": [
              "ReturnedOk"
            ],
            "type": "string"
          },
          {
            "properties": {
              "ReturnedError": {
                "$ref": "#/components/schemas/ServiceFailureClass"
              }
            },
            "required": [
              "ReturnedError"
            ],
            "type": "object"
          }
        ]
      },
      "BatchId": {
        "description": "Durable batch correlation. No provider-side idempotency is assumed.",
        "type": "string"
      },
      "BindingId": {
        "description": "Fresh attachment token on every detach/reattach or control-owner replacement.",
        "type": "string"
      },
      "BrowserRead": {
        "oneOf": [
          {
            "enum": [
              "Snapshot"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Requests": {
                "properties": {
                  "page": {
                    "$ref": "#/components/schemas/RuntimeList"
                  },
                  "select": {
                    "$ref": "#/components/schemas/RequestSelection"
                  }
                },
                "required": [
                  "select",
                  "page"
                ],
                "type": "object"
              }
            },
            "required": [
              "Requests"
            ],
            "type": "object"
          },
          {
            "properties": {
              "RequestDetail": {
                "$ref": "#/components/schemas/RequestObservationRef"
              }
            },
            "required": [
              "RequestDetail"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Connections": {
                "$ref": "#/components/schemas/RuntimeList"
              }
            },
            "required": [
              "Connections"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Peers": {
                "$ref": "#/components/schemas/RuntimeList"
              }
            },
            "required": [
              "Peers"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Hosts": {
                "$ref": "#/components/schemas/RuntimeList"
              }
            },
            "required": [
              "Hosts"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ClientReports": {
                "$ref": "#/components/schemas/RuntimeList"
              }
            },
            "required": [
              "ClientReports"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ClientReportDetail": {
                "$ref": "#/components/schemas/ClientReportRef"
              }
            },
            "required": [
              "ClientReportDetail"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ConfigurationHead": {
                "properties": {
                  "expected": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/ObservationEpoch"
                      }
                    ]
                  }
                },
                "type": "object"
              }
            },
            "required": [
              "ConfigurationHead"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ConfigurationLookup": {
                "$ref": "#/components/schemas/ConfigSnapshotRef"
              }
            },
            "required": [
              "ConfigurationLookup"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ConfigurationCompare": {
                "$ref": "#/components/schemas/CompareConfig"
              }
            },
            "required": [
              "ConfigurationCompare"
            ],
            "type": "object"
          },
          {
            "enum": [
              "EventsDescribe"
            ],
            "type": "string"
          },
          {
            "properties": {
              "EventsRead": {
                "$ref": "#/components/schemas/ObservationRead"
              }
            },
            "required": [
              "EventsRead"
            ],
            "type": "object"
          }
        ]
      },
      "CandidateDisposition": {
        "oneOf": [
          {
            "enum": [
              "Selected"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Shadowed"
            ],
            "type": "string"
          },
          {
            "enum": [
              "IgnoredEmpty"
            ],
            "type": "string"
          },
          {
            "enum": [
              "IgnoredByFieldPolicy"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Rejected"
            ],
            "type": "string"
          },
          {
            "enum": [
              "NotConsulted"
            ],
            "type": "string"
          },
          {
            "properties": {
              "NotObserved": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "NotObserved"
            ],
            "type": "object"
          }
        ]
      },
      "CandidateValidation": {
        "oneOf": [
          {
            "enum": [
              "Valid"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Invalid": {
                "$ref": "#/components/schemas/ConfigIssueCode"
              }
            },
            "required": [
              "Invalid"
            ],
            "type": "object"
          },
          {
            "enum": [
              "NotEvaluated"
            ],
            "type": "string"
          }
        ]
      },
      "CaptureSkipReason": {
        "enum": [
          "Pressure",
          "Oversize",
          "Unsupported",
          "ObservationFailed",
          "StaleEpoch",
          "GenerationUnavailable"
        ],
        "type": "string"
      },
      "ClaimPhase": {
        "enum": [
          "Prepared",
          "Issuing",
          "Admitted",
          "Uncertain",
          "Blocked"
        ],
        "type": "string"
      },
      "ClientAttemptId": {
        "type": "string"
      },
      "ClientAttemptKind": {
        "enum": [
          "Acquire",
          "Probe",
          "Call"
        ],
        "type": "string"
      },
      "ClientAttemptReport": {
        "properties": {
          "attempt": {
            "$ref": "#/components/schemas/ClientAttemptId"
          },
          "ended_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "failure": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/LocalFailure"
              }
            ]
          },
          "invocation": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/InvocationCaptureRef"
              }
            ]
          },
          "kind": {
            "$ref": "#/components/schemas/ClientAttemptKind"
          },
          "knowledge": {
            "$ref": "#/components/schemas/ClientCallKnowledge"
          },
          "last_ready": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/InspectionReady"
              }
            ]
          },
          "omitted_stages": {
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
          "original_epoch": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationEpoch"
              }
            ]
          },
          "request_id": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestId"
              }
            ]
          },
          "requested_scope": {
            "type": "string"
          },
          "resolved_scope": {
            "$ref": "#/components/schemas/EvidenceValue_ScopeRef"
          },
          "stages": {
            "items": {
              "$ref": "#/components/schemas/ClientStageSpan"
            },
            "type": "array"
          },
          "started_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "attempt",
          "kind",
          "requested_scope",
          "resolved_scope",
          "started_at",
          "ended_at",
          "stages",
          "omitted_stages",
          "knowledge"
        ],
        "type": "object"
      },
      "ClientCallKnowledge": {
        "oneOf": [
          {
            "enum": [
              "NotWritten"
            ],
            "type": "string"
          },
          {
            "enum": [
              "MayHaveBeenWritten"
            ],
            "type": "string"
          },
          {
            "properties": {
              "ReceivedFailure": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "ReceivedFailure"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ReceivedApplicationResult": {
                "$ref": "#/components/schemas/ApplicationResultClass"
              }
            },
            "required": [
              "ReceivedApplicationResult"
            ],
            "type": "object"
          },
          {
            "enum": [
              "DiscoveryOnly"
            ],
            "type": "string"
          }
        ]
      },
      "ClientInvocationId": {
        "type": "string"
      },
      "ClientReportId": {
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
      "ClientReportObservation": {
        "description": "Current submitter is authenticated separately from the claimed historic origin.",
        "properties": {
          "report": {
            "$ref": "#/components/schemas/ClientAttemptReport"
          },
          "submitted_by": {
            "$ref": "#/components/schemas/AcceptedPeer"
          }
        },
        "required": [
          "report",
          "submitted_by"
        ],
        "type": "object"
      },
      "ClientReportReceipt": {
        "properties": {
          "attempt": {
            "$ref": "#/components/schemas/ClientAttemptId"
          },
          "event": {
            "$ref": "#/components/schemas/RecordDisposition"
          },
          "received_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "reference": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ClientReportRef"
              }
            ]
          },
          "retained": {
            "type": "boolean"
          }
        },
        "required": [
          "attempt",
          "received_at",
          "retained",
          "event"
        ],
        "type": "object"
      },
      "ClientReportRef": {
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "id": {
            "$ref": "#/components/schemas/ClientReportId"
          }
        },
        "required": [
          "daemon",
          "id"
        ],
        "type": "object"
      },
      "ClientStage": {
        "enum": [
          "Capture",
          "ScopeLookup",
          "EndpointLookup",
          "Discovery",
          "StartupLock",
          "Spawn",
          "Connect",
          "PeerAuthentication",
          "Hello",
          "CallWrite",
          "ReplyRead",
          "InspectionRead",
          "InspectionWrite"
        ],
        "type": "string"
      },
      "ClientStageSpan": {
        "properties": {
          "ended_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "failure": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/LocalFailureKind"
              }
            ]
          },
          "stage": {
            "$ref": "#/components/schemas/ClientStage"
          },
          "started_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "stage",
          "started_at",
          "ended_at"
        ],
        "type": "object"
      },
      "ClockId": {
        "description": "UUID v4 identifying one retained process-clock origin, never OS authority.",
        "type": "string"
      },
      "CollectionLosses": {
        "properties": {
          "dropped": {
            "$ref": "#/components/schemas/DroppedCounts"
          },
          "evicted": {
            "$ref": "#/components/schemas/EvictedCounts"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "suppressed": {
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
          "sampled_at",
          "dropped",
          "evicted",
          "suppressed"
        ],
        "type": "object"
      },
      "CompareConfig": {
        "properties": {
          "fields": {
            "$ref": "#/components/schemas/ConfigFieldSelection"
          },
          "left": {
            "$ref": "#/components/schemas/ConfigSnapshotRef"
          },
          "right": {
            "$ref": "#/components/schemas/ConfigSnapshotRef"
          }
        },
        "required": [
          "left",
          "right",
          "fields"
        ],
        "type": "object"
      },
      "ComparisonBasis": {
        "properties": {
          "left": {
            "$ref": "#/components/schemas/ConfigSnapshotRef"
          },
          "left_captured_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "left_provenance": {
            "$ref": "#/components/schemas/ConfigProvenance"
          },
          "left_purpose": {
            "$ref": "#/components/schemas/ConfigPurpose"
          },
          "left_resolution_scope": {
            "$ref": "#/components/schemas/EvidenceValue_ScopeRef"
          },
          "left_rules": {
            "$ref": "#/components/schemas/RulesRevision"
          },
          "left_subject": {
            "$ref": "#/components/schemas/ConfigSubject"
          },
          "right": {
            "$ref": "#/components/schemas/ConfigSnapshotRef"
          },
          "right_captured_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "right_provenance": {
            "$ref": "#/components/schemas/ConfigProvenance"
          },
          "right_purpose": {
            "$ref": "#/components/schemas/ConfigPurpose"
          },
          "right_resolution_scope": {
            "$ref": "#/components/schemas/EvidenceValue_ScopeRef"
          },
          "right_rules": {
            "$ref": "#/components/schemas/RulesRevision"
          },
          "right_subject": {
            "$ref": "#/components/schemas/ConfigSubject"
          },
          "stage": {
            "$ref": "#/components/schemas/ConfigStage"
          }
        },
        "required": [
          "left",
          "right",
          "left_subject",
          "right_subject",
          "left_resolution_scope",
          "right_resolution_scope",
          "stage",
          "left_purpose",
          "right_purpose",
          "left_captured_at",
          "right_captured_at",
          "left_provenance",
          "right_provenance",
          "left_rules",
          "right_rules"
        ],
        "type": "object"
      },
      "ComparisonBoundary": {
        "enum": [
          "DifferentScope",
          "ScopeUnestablished",
          "DifferentEpoch",
          "DifferentStage",
          "DifferentMeaningRevision"
        ],
        "type": "string"
      },
      "ComparisonUnknown": {
        "enum": [
          "Withheld",
          "Unobserved",
          "CaptureUnavailable",
          "InvalidValue"
        ],
        "type": "string"
      },
      "ConfigCaptureResult": {
        "oneOf": [
          {
            "properties": {
              "Recorded": {
                "$ref": "#/components/schemas/ConfigSnapshotRef"
              }
            },
            "required": [
              "Recorded"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Skipped": {
                "$ref": "#/components/schemas/CaptureSkipReason"
              }
            },
            "required": [
              "Skipped"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigCorrelation": {
        "properties": {
          "attempt": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ClientAttemptId"
              }
            ]
          },
          "invocation": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/InvocationCaptureRef"
              }
            ]
          }
        },
        "type": "object"
      },
      "ConfigFieldId": {
        "oneOf": [
          {
            "properties": {
              "Setting": {
                "$ref": "#/components/schemas/ConfigSettingId"
              }
            },
            "required": [
              "Setting"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Context": {
                "$ref": "#/components/schemas/ContextFieldId"
              }
            },
            "required": [
              "Context"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Participant": {
                "$ref": "#/components/schemas/ParticipantFieldId"
              }
            },
            "required": [
              "Participant"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigFieldSelection": {
        "oneOf": [
          {
            "enum": [
              "All"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Only": {
                "items": {
                  "$ref": "#/components/schemas/ConfigFieldId"
                },
                "type": "array"
              }
            },
            "required": [
              "Only"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigIssueCode": {
        "enum": [
          "WrongScalarType",
          "InvalidInteger",
          "InvalidBoolean",
          "OutOfRange",
          "InvalidEnum",
          "InvalidSyntax",
          "UnsafeTerminal",
          "UnsupportedTerminal",
          "SourceUnreadable",
          "SourceSyntaxInvalid",
          "ValidationNotObserved"
        ],
        "type": "string"
      },
      "ConfigMissing": {
        "oneOf": [
          {
            "enum": [
              "NeverIssued"
            ],
            "type": "string"
          },
          {
            "properties": {
              "NoLongerRetained": {
                "properties": {
                  "retired_through": {
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
                  "retired_through"
                ],
                "type": "object"
              }
            },
            "required": [
              "NoLongerRetained"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigObservationEvent": {
        "oneOf": [
          {
            "properties": {
              "SnapshotPublished": {
                "properties": {
                  "attempt": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/ClientAttemptId"
                      }
                    ]
                  },
                  "captured_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  },
                  "invocation": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/InvocationCaptureRef"
                      }
                    ]
                  },
                  "origin": {
                    "$ref": "#/components/schemas/EvidenceOrigin"
                  },
                  "snapshot": {
                    "$ref": "#/components/schemas/ConfigSnapshotRef"
                  },
                  "stage": {
                    "$ref": "#/components/schemas/ConfigStage"
                  }
                },
                "required": [
                  "snapshot",
                  "stage",
                  "origin",
                  "captured_at"
                ],
                "type": "object"
              }
            },
            "required": [
              "SnapshotPublished"
            ],
            "type": "object"
          },
          {
            "properties": {
              "CaptureSkipped": {
                "properties": {
                  "attempt": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/ClientAttemptId"
                      }
                    ]
                  },
                  "invocation": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/InvocationCaptureRef"
                      }
                    ]
                  },
                  "reason": {
                    "$ref": "#/components/schemas/CaptureSkipReason"
                  },
                  "stage": {
                    "$ref": "#/components/schemas/ConfigStage"
                  }
                },
                "required": [
                  "stage",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "CaptureSkipped"
            ],
            "type": "object"
          },
          {
            "properties": {
              "SnapshotsRetired": {
                "properties": {
                  "count": {
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
                  "reason": {
                    "$ref": "#/components/schemas/RetentionEnd"
                  },
                  "through_sequence": {
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
                  "through_sequence",
                  "count",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "SnapshotsRetired"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigProvenance": {
        "oneOf": [
          {
            "enum": [
              "DaemonCaptured"
            ],
            "type": "string"
          },
          {
            "enum": [
              "ApplicationResolved"
            ],
            "type": "string"
          },
          {
            "enum": [
              "StoredDomainProjection"
            ],
            "type": "string"
          },
          {
            "properties": {
              "ClientReported": {
                "properties": {
                  "peer": {
                    "$ref": "#/components/schemas/AcceptedPeer"
                  },
                  "source": {
                    "$ref": "#/components/schemas/LocalConfigSnapshotRef"
                  }
                },
                "required": [
                  "source",
                  "peer"
                ],
                "type": "object"
              }
            },
            "required": [
              "ClientReported"
            ],
            "type": "object"
          },
          {
            "properties": {
              "HostReported": {
                "properties": {
                  "peer": {
                    "$ref": "#/components/schemas/AcceptedPeer"
                  }
                },
                "required": [
                  "peer"
                ],
                "type": "object"
              }
            },
            "required": [
              "HostReported"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigPurpose": {
        "enum": [
          "StartupCapture",
          "InputCapture",
          "DeclarationRead",
          "ActualResolution",
          "LaunchPreparation",
          "ValidatedHostReport",
          "DomainProjection",
          "DiagnosticReevaluation"
        ],
        "type": "string"
      },
      "ConfigReportError": {
        "oneOf": [
          {
            "enum": [
              "UnsupportedRevision"
            ],
            "type": "string"
          },
          {
            "enum": [
              "InvalidSubject"
            ],
            "type": "string"
          },
          {
            "enum": [
              "InvalidFields"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Oversize"
            ],
            "type": "string"
          },
          {
            "properties": {
              "WrongEpoch": {
                "properties": {
                  "current": {
                    "$ref": "#/components/schemas/ObservationEpoch"
                  },
                  "requested": {
                    "$ref": "#/components/schemas/ObservationEpoch"
                  }
                },
                "required": [
                  "requested",
                  "current"
                ],
                "type": "object"
              }
            },
            "required": [
              "WrongEpoch"
            ],
            "type": "object"
          },
          {
            "properties": {
              "GenerationUnavailable": {
                "$ref": "#/components/schemas/DaemonRef"
              }
            },
            "required": [
              "GenerationUnavailable"
            ],
            "type": "object"
          },
          {
            "enum": [
              "PeerQuota"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Pressure"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Unavailable"
            ],
            "type": "string"
          }
        ]
      },
      "ConfigSettingId": {
        "type": "string"
      },
      "ConfigSnapshot": {
        "properties": {
          "body": {
            "$ref": "#/components/schemas/ConfigSnapshotBody"
          },
          "provenance": {
            "$ref": "#/components/schemas/ConfigProvenance"
          },
          "reference": {
            "$ref": "#/components/schemas/ConfigSnapshotRef"
          },
          "retained_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "reference",
          "body",
          "provenance",
          "retained_at"
        ],
        "type": "object"
      },
      "ConfigSnapshotBody": {
        "properties": {
          "captured_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "consistency": {
            "$ref": "#/components/schemas/ReadConsistency"
          },
          "consumed_fields": {
            "items": {
              "$ref": "#/components/schemas/ConfigFieldId"
            },
            "type": "array"
          },
          "correlation": {
            "$ref": "#/components/schemas/ConfigCorrelation"
          },
          "fields": {
            "items": {
              "$ref": "#/components/schemas/FieldEvidence"
            },
            "type": "array"
          },
          "purpose": {
            "$ref": "#/components/schemas/ConfigPurpose"
          },
          "resolution_scope": {
            "$ref": "#/components/schemas/EvidenceValue_ScopeRef"
          },
          "rules": {
            "$ref": "#/components/schemas/RulesRevision"
          },
          "source_reads": {
            "items": {
              "$ref": "#/components/schemas/SourceRead"
            },
            "type": "array"
          },
          "stage": {
            "$ref": "#/components/schemas/ConfigStage"
          },
          "subject": {
            "$ref": "#/components/schemas/ConfigSubject"
          }
        },
        "required": [
          "subject",
          "resolution_scope",
          "stage",
          "purpose",
          "correlation",
          "rules",
          "captured_at",
          "consistency",
          "source_reads",
          "fields",
          "consumed_fields"
        ],
        "type": "object"
      },
      "ConfigSnapshotRef": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "sequence": {
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
          "epoch",
          "sequence"
        ],
        "type": "object"
      },
      "ConfigSourceKind": {
        "enum": [
          "BuiltInDefault",
          "InvocationEnvironment",
          "ScopeFile",
          "ParsedOption",
          "CapturedParent",
          "CleanShell",
          "LaunchEnvironmentFile",
          "ExplicitLaunchEnvironment",
          "DomainRecord",
          "ToolResolution",
          "ReservedIdentityInjection"
        ],
        "type": "string"
      },
      "ConfigSourceRef": {
        "properties": {
          "kind": {
            "$ref": "#/components/schemas/ConfigSourceKind"
          },
          "read": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/SourceReadId"
              }
            ]
          }
        },
        "required": [
          "kind"
        ],
        "type": "object"
      },
      "ConfigStage": {
        "enum": [
          "StartupInputs",
          "InvocationInputs",
          "ScopedDeclarations",
          "ScopedEffective",
          "LaunchPlanned",
          "HostReported",
          "ParticipantState"
        ],
        "type": "string"
      },
      "ConfigSubject": {
        "oneOf": [
          {
            "properties": {
              "DaemonStartup": {
                "properties": {
                  "daemon": {
                    "$ref": "#/components/schemas/DaemonRef"
                  }
                },
                "required": [
                  "daemon"
                ],
                "type": "object"
              }
            },
            "required": [
              "DaemonStartup"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Invocation": {
                "properties": {
                  "capture": {
                    "$ref": "#/components/schemas/InvocationCaptureRef"
                  }
                },
                "required": [
                  "capture"
                ],
                "type": "object"
              }
            },
            "required": [
              "Invocation"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ScopeDeclarations": {
                "properties": {
                  "scope": {
                    "$ref": "#/components/schemas/ScopeRef"
                  }
                },
                "required": [
                  "scope"
                ],
                "type": "object"
              }
            },
            "required": [
              "ScopeDeclarations"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Participant": {
                "properties": {
                  "participant": {
                    "$ref": "#/components/schemas/ParticipantRef"
                  }
                },
                "required": [
                  "participant"
                ],
                "type": "object"
              }
            },
            "required": [
              "Participant"
            ],
            "type": "object"
          },
          {
            "properties": {
              "HostPlan": {
                "properties": {
                  "capture": {
                    "$ref": "#/components/schemas/InvocationCaptureRef"
                  }
                },
                "required": [
                  "capture"
                ],
                "type": "object"
              }
            },
            "required": [
              "HostPlan"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Host": {
                "properties": {
                  "execution": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/ExecutionId"
                      }
                    ]
                  },
                  "participant": {
                    "$ref": "#/components/schemas/ParticipantRef"
                  }
                },
                "required": [
                  "participant"
                ],
                "type": "object"
              }
            },
            "required": [
              "Host"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigurationHead": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "last_issued": {
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
          "limits": {
            "$ref": "#/components/schemas/ConfigurationLimits"
          },
          "observed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "oldest_retained": {
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
          "retained_count": {
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
          "retained_payload_bytes": {
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
          "retired_through": {
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
          "skipped_other": {
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
          "skipped_oversize": {
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
          "skipped_pressure": {
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
          "epoch",
          "observed_at",
          "last_issued",
          "retired_through",
          "retained_count",
          "retained_payload_bytes",
          "skipped_pressure",
          "skipped_oversize",
          "skipped_other",
          "limits"
        ],
        "type": "object"
      },
      "ConfigurationLimits": {
        "properties": {
          "fields_per_snapshot": {
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
            "x-agentx-native-integer": "u-int16",
            "x-agentx-native-schema": {
              "format": "u-int16",
              "minimum": 0,
              "type": "integer"
            }
          },
          "max_age_ms": {
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
          "payload_bytes": {
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
          "snapshot_bytes": {
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
          "snapshots": {
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
          "source_reads_per_snapshot": {
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
            "x-agentx-native-integer": "u-int8",
            "x-agentx-native-schema": {
              "format": "u-int8",
              "minimum": 0,
              "type": "integer"
            }
          },
          "steps_per_field": {
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
            "x-agentx-native-integer": "u-int8",
            "x-agentx-native-schema": {
              "format": "u-int8",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "snapshots",
          "payload_bytes",
          "max_age_ms",
          "snapshot_bytes",
          "fields_per_snapshot",
          "steps_per_field",
          "source_reads_per_snapshot"
        ],
        "type": "object"
      },
      "ConfigurationQueryError": {
        "oneOf": [
          {
            "properties": {
              "GenerationUnavailable": {
                "$ref": "#/components/schemas/DaemonRef"
              }
            },
            "required": [
              "GenerationUnavailable"
            ],
            "type": "object"
          },
          {
            "properties": {
              "EpochChanged": {
                "properties": {
                  "current": {
                    "$ref": "#/components/schemas/ObservationEpoch"
                  },
                  "requested": {
                    "$ref": "#/components/schemas/ObservationEpoch"
                  }
                },
                "required": [
                  "requested",
                  "current"
                ],
                "type": "object"
              }
            },
            "required": [
              "EpochChanged"
            ],
            "type": "object"
          },
          {
            "enum": [
              "InvalidReference"
            ],
            "type": "string"
          },
          {
            "enum": [
              "UnknownField"
            ],
            "type": "string"
          },
          {
            "enum": [
              "InvalidSelection"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Oversize"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Busy"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Unavailable"
            ],
            "type": "string"
          }
        ]
      },
      "ConfigurationRead_CompareConfigResult": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "observed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "result": {
            "oneOf": [
              {
                "properties": {
                  "Compared": {
                    "properties": {
                      "basis": {
                        "$ref": "#/components/schemas/ComparisonBasis"
                      },
                      "fields": {
                        "items": {
                          "$ref": "#/components/schemas/FieldComparison"
                        },
                        "type": "array"
                      }
                    },
                    "required": [
                      "basis",
                      "fields"
                    ],
                    "type": "object"
                  }
                },
                "required": [
                  "Compared"
                ],
                "type": "object"
              },
              {
                "properties": {
                  "Unavailable": {
                    "properties": {
                      "left": {
                        "$ref": "#/components/schemas/SnapshotLookup"
                      },
                      "right": {
                        "$ref": "#/components/schemas/SnapshotLookup"
                      }
                    },
                    "required": [
                      "left",
                      "right"
                    ],
                    "type": "object"
                  }
                },
                "required": [
                  "Unavailable"
                ],
                "type": "object"
              },
              {
                "properties": {
                  "Incomparable": {
                    "$ref": "#/components/schemas/ComparisonBoundary"
                  }
                },
                "required": [
                  "Incomparable"
                ],
                "type": "object"
              }
            ]
          }
        },
        "required": [
          "epoch",
          "observed_at",
          "result"
        ],
        "type": "object"
      },
      "ConfigurationRead_SnapshotLookup": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "observed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "result": {
            "oneOf": [
              {
                "properties": {
                  "Found": {
                    "$ref": "#/components/schemas/ConfigSnapshot"
                  }
                },
                "required": [
                  "Found"
                ],
                "type": "object"
              },
              {
                "properties": {
                  "Missing": {
                    "properties": {
                      "reason": {
                        "$ref": "#/components/schemas/ConfigMissing"
                      },
                      "reference": {
                        "$ref": "#/components/schemas/ConfigSnapshotRef"
                      }
                    },
                    "required": [
                      "reference",
                      "reason"
                    ],
                    "type": "object"
                  }
                },
                "required": [
                  "Missing"
                ],
                "type": "object"
              }
            ]
          }
        },
        "required": [
          "epoch",
          "observed_at",
          "result"
        ],
        "type": "object"
      },
      "ConfigurationTrace": {
        "oneOf": [
          {
            "enum": [
              "NotEvaluated"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Captured": {
                "properties": {
                  "omitted_uses": {
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
                  "uses": {
                    "items": {
                      "$ref": "#/components/schemas/ConfigurationUse"
                    },
                    "type": "array"
                  }
                },
                "required": [
                  "uses",
                  "omitted_uses"
                ],
                "type": "object"
              }
            },
            "required": [
              "Captured"
            ],
            "type": "object"
          }
        ]
      },
      "ConfigurationUse": {
        "properties": {
          "capture": {
            "$ref": "#/components/schemas/ConfigCaptureResult"
          },
          "stage": {
            "$ref": "#/components/schemas/ConfigStage"
          }
        },
        "required": [
          "stage",
          "capture"
        ],
        "type": "object"
      },
      "ConfirmationDisposition": {
        "enum": [
          "Advanced",
          "AlreadyConfirmed"
        ],
        "type": "string"
      },
      "ConnectionCloseReason": {
        "enum": [
          "HelloOnlyEof",
          "Completed",
          "ReadFailure",
          "WriteFailure",
          "Deadline",
          "Rejected",
          "Shutdown"
        ],
        "type": "string"
      },
      "ConnectionId": {
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
      "ConnectionPhase": {
        "enum": [
          "Accepted",
          "PeerValidated",
          "HelloValidated",
          "ReadyWritten",
          "ReadingOperation",
          "AwaitingReply",
          "Closed"
        ],
        "type": "string"
      },
      "ConnectionRef": {
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "id": {
            "$ref": "#/components/schemas/ConnectionId"
          }
        },
        "required": [
          "daemon",
          "id"
        ],
        "type": "object"
      },
      "ContextFieldId": {
        "enum": [
          "Cwd",
          "RequestedScope",
          "CanonicalScope",
          "StdinIsTty",
          "StdoutIsTty",
          "ExplicitNameSelection",
          "ProviderHome",
          "Tool",
          "LaunchRegime",
          "LaunchBackend",
          "EffectiveWorkingDirectory",
          "EffectiveDevRoot"
        ],
        "type": "string"
      },
      "DaemonLifecycle": {
        "enum": [
          "Initializing",
          "Serving",
          "Draining",
          "Stopping",
          "Failed"
        ],
        "type": "string"
      },
      "DaemonRef": {
        "description": "One daemon incarnation within a canonical scope; independent of PID/build.",
        "properties": {
          "daemon_instance": {
            "type": "string"
          },
          "scope": {
            "$ref": "#/components/schemas/ScopeRef"
          }
        },
        "required": [
          "scope",
          "daemon_instance"
        ],
        "type": "object"
      },
      "DeliveryBoundary": {
        "enum": [
          "HookOutputWritten",
          "ToolOutputWritten",
          "ProviderInputRecorded"
        ],
        "type": "string"
      },
      "DispatchActivity": {
        "oneOf": [
          {
            "enum": [
              "Initializing"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Idle"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Running": {
                "$ref": "#/components/schemas/RequestObservationRef"
              }
            },
            "required": [
              "Running"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Draining": {
                "oneOf": [
                  {
                    "type": "null"
                  },
                  {
                    "$ref": "#/components/schemas/RequestObservationRef"
                  }
                ]
              }
            },
            "required": [
              "Draining"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Unavailable"
            ],
            "type": "string"
          }
        ]
      },
      "DomainCommitFact": {
        "oneOf": [
          {
            "properties": {
              "EventAppended": {
                "properties": {
                  "event": {
                    "$ref": "#/components/schemas/DurableEventRef"
                  },
                  "kind": {
                    "$ref": "#/components/schemas/DomainEventKind"
                  }
                },
                "required": [
                  "event",
                  "kind"
                ],
                "type": "object"
              }
            },
            "required": [
              "EventAppended"
            ],
            "type": "object"
          },
          {
            "properties": {
              "MessageAccepted": {
                "properties": {
                  "acceptance": {
                    "$ref": "#/components/schemas/MessageAcceptanceRef"
                  },
                  "event": {
                    "$ref": "#/components/schemas/DurableEventRef"
                  },
                  "replayed": {
                    "type": "boolean"
                  }
                },
                "required": [
                  "acceptance",
                  "event",
                  "replayed"
                ],
                "type": "object"
              }
            },
            "required": [
              "MessageAccepted"
            ],
            "type": "object"
          },
          {
            "properties": {
              "InboxPhaseStored": {
                "properties": {
                  "claim": {
                    "$ref": "#/components/schemas/InboxClaimRef"
                  },
                  "phase": {
                    "$ref": "#/components/schemas/ClaimPhase"
                  }
                },
                "required": [
                  "claim",
                  "phase"
                ],
                "type": "object"
              }
            },
            "required": [
              "InboxPhaseStored"
            ],
            "type": "object"
          },
          {
            "properties": {
              "DeliveryConfirmationStored": {
                "properties": {
                  "boundary": {
                    "$ref": "#/components/schemas/DeliveryBoundary"
                  },
                  "claim": {
                    "$ref": "#/components/schemas/InboxClaimRef"
                  },
                  "confirmation": {
                    "$ref": "#/components/schemas/ConfirmationDisposition"
                  }
                },
                "required": [
                  "claim",
                  "boundary",
                  "confirmation"
                ],
                "type": "object"
              }
            },
            "required": [
              "DeliveryConfirmationStored"
            ],
            "type": "object"
          },
          {
            "properties": {
              "OutputConfirmationStored": {
                "properties": {
                  "output": {
                    "$ref": "#/components/schemas/OutputEvidenceRef"
                  },
                  "through": {
                    "$ref": "#/components/schemas/EventId"
                  }
                },
                "required": [
                  "output",
                  "through"
                ],
                "type": "object"
              }
            },
            "required": [
              "OutputConfirmationStored"
            ],
            "type": "object"
          }
        ]
      },
      "DomainCommitObservation": {
        "properties": {
          "correlation": {
            "$ref": "#/components/schemas/DomainCorrelation"
          },
          "fact": {
            "$ref": "#/components/schemas/DomainCommitFact"
          }
        },
        "required": [
          "correlation",
          "fact"
        ],
        "type": "object"
      },
      "DomainCorrelation": {
        "properties": {
          "actor": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ParticipantRef"
              }
            ]
          },
          "client_attempt": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ClientAttemptId"
              }
            ]
          },
          "request": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestObservationId"
              }
            ]
          },
          "request_id": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestId"
              }
            ]
          }
        },
        "type": "object"
      },
      "DomainEventKind": {
        "enum": [
          "Message",
          "Status",
          "Life",
          "Other"
        ],
        "type": "string"
      },
      "DropReason": {
        "enum": [
          "Contended",
          "Oversize",
          "InvalidInput",
          "RetiredEpoch",
          "CollectorUnavailable",
          "PreparationBudget"
        ],
        "type": "string"
      },
      "DroppedCounts": {
        "properties": {
          "collector_unavailable": {
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
          "contended": {
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
          "invalid_input": {
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
          "oversize": {
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
          "preparation_budget": {
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
          "retired_epoch": {
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
          "contended",
          "oversize",
          "invalid_input",
          "retired_epoch",
          "collector_unavailable",
          "preparation_budget"
        ],
        "type": "object"
      },
      "DurableEventRef": {
        "properties": {
          "database_generation": {
            "type": "string"
          },
          "event": {
            "$ref": "#/components/schemas/EventId"
          },
          "scope": {
            "$ref": "#/components/schemas/ScopeRef"
          }
        },
        "required": [
          "scope",
          "database_generation",
          "event"
        ],
        "type": "object"
      },
      "Empty": {
        "additionalProperties": false,
        "type": "object"
      },
      "EventId": {
        "additionalProperties": false,
        "description": "Local SQLite event id, ordered only inside this store.",
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
          "description": "Local SQLite event id, ordered only inside this store.",
          "format": "int64",
          "type": "integer"
        }
      },
      "EvictedCounts": {
        "properties": {
          "age": {
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
          "byte_pressure": {
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
          "count_pressure": {
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
          "age",
          "count_pressure",
          "byte_pressure"
        ],
        "type": "object"
      },
      "EvidenceOrigin": {
        "description": "Where a fact was obtained; provenance never implies authentication.",
        "enum": [
          "OsObserved",
          "DaemonBoundary",
          "ApplicationResolved",
          "StoredDomainFact",
          "ClientReported",
          "HostReported",
          "Derived"
        ],
        "type": "string"
      },
      "EvidenceStamp": {
        "description": "Original producer capture/attempt time, distinct from journal retention time.",
        "properties": {
          "at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "origin": {
            "$ref": "#/components/schemas/EvidenceOrigin"
          }
        },
        "required": [
          "origin",
          "at"
        ],
        "type": "object"
      },
      "EvidenceValue_ConfigValue": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "oneOf": [
                  {
                    "properties": {
                      "Text": {
                        "type": "string"
                      }
                    },
                    "required": [
                      "Text"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Boolean": {
                        "type": "boolean"
                      }
                    },
                    "required": [
                      "Boolean"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Seconds": {
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
                      "Seconds"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Path": {
                        "type": "string"
                      }
                    },
                    "required": [
                      "Path"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "EvidenceValue_FieldResolution": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "oneOf": [
                  {
                    "enum": [
                      "InputOnly"
                    ],
                    "type": "string"
                  },
                  {
                    "properties": {
                      "Effective": {
                        "properties": {
                          "normalized": {
                            "type": "boolean"
                          },
                          "source": {
                            "$ref": "#/components/schemas/ConfigSourceRef"
                          }
                        },
                        "required": [
                          "source",
                          "normalized"
                        ],
                        "type": "object"
                      }
                    },
                    "required": [
                      "Effective"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Defaulted": {
                        "properties": {
                          "issue": {
                            "$ref": "#/components/schemas/ConfigIssueCode"
                          },
                          "rejected": {
                            "$ref": "#/components/schemas/ConfigSourceRef"
                          }
                        },
                        "required": [
                          "rejected",
                          "issue"
                        ],
                        "type": "object"
                      }
                    },
                    "required": [
                      "Defaulted"
                    ],
                    "type": "object"
                  },
                  {
                    "enum": [
                      "Absent"
                    ],
                    "type": "string"
                  },
                  {
                    "properties": {
                      "Unknown": {
                        "$ref": "#/components/schemas/UnknownReason"
                      }
                    },
                    "required": [
                      "Unknown"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "EvidenceValue_ObservationTime": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "description": "A producer's clock-domain time; unavailable wall time remains None.",
                "properties": {
                  "clock": {
                    "$ref": "#/components/schemas/ClockId"
                  },
                  "monotonic_ms": {
                    "$ref": "#/components/schemas/MonotonicMillis"
                  },
                  "unix_ms": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/UnixMillis"
                      }
                    ]
                  }
                },
                "required": [
                  "clock",
                  "monotonic_ms"
                ],
                "type": "object"
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "EvidenceValue_PeerObservationRef": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "properties": {
                  "daemon": {
                    "$ref": "#/components/schemas/DaemonRef"
                  },
                  "id": {
                    "$ref": "#/components/schemas/PeerObservationId"
                  }
                },
                "required": [
                  "daemon",
                  "id"
                ],
                "type": "object"
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "EvidenceValue_ScopeRef": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "description": "Canonical local data scope, supplied by an existing resolver or lease.",
                "properties": {
                  "canonical_scope": {
                    "type": "string"
                  }
                },
                "required": [
                  "canonical_scope"
                ],
                "type": "object"
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "EvidenceValue_String": {
        "description": "Absent requires a complete positive lookup; Unknown/Withheld prove no absence.",
        "oneOf": [
          {
            "properties": {
              "Present": {
                "type": "string"
              }
            },
            "required": [
              "Present"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Absent"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Withheld"
            ],
            "type": "string"
          }
        ]
      },
      "Evidence_String": {
        "description": "A captured value with source provenance; freshness policy belongs to its owner.",
        "properties": {
          "stamp": {
            "$ref": "#/components/schemas/EvidenceStamp"
          },
          "value": {
            "$ref": "#/components/schemas/EvidenceValue_String"
          }
        },
        "required": [
          "value",
          "stamp"
        ],
        "type": "object"
      },
      "ExecutionId": {
        "description": "Runtime generation. Shared hosting uses the backend's stable generation;\nother providers retain their existing launcher execution scope.",
        "type": "string"
      },
      "ExecutionObservation": {
        "oneOf": [
          {
            "enum": [
              "Decoded"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Queued": {
                "properties": {
                  "enqueued_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "enqueued_at"
                ],
                "type": "object"
              }
            },
            "required": [
              "Queued"
            ],
            "type": "object"
          },
          {
            "properties": {
              "NotRun": {
                "properties": {
                  "at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  },
                  "reason": {
                    "$ref": "#/components/schemas/NotRunReason"
                  }
                },
                "required": [
                  "at",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "NotRun"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Running": {
                "properties": {
                  "started_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "started_at"
                ],
                "type": "object"
              }
            },
            "required": [
              "Running"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Returned": {
                "properties": {
                  "ended_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  },
                  "result": {
                    "$ref": "#/components/schemas/ApplicationResultClass"
                  },
                  "started_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "started_at",
                  "ended_at",
                  "result"
                ],
                "type": "object"
              }
            },
            "required": [
              "Returned"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Interrupted": {
                "properties": {
                  "last_observed_at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "last_observed_at"
                ],
                "type": "object"
              }
            },
            "required": [
              "Interrupted"
            ],
            "type": "object"
          }
        ]
      },
      "FieldComparison": {
        "properties": {
          "field": {
            "$ref": "#/components/schemas/ConfigFieldId"
          },
          "left": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/FieldEvidence"
              }
            ]
          },
          "provenance": {
            "$ref": "#/components/schemas/ProvenanceRelation"
          },
          "right": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/FieldEvidence"
              }
            ]
          },
          "value": {
            "$ref": "#/components/schemas/ValueRelation"
          }
        },
        "required": [
          "field",
          "value",
          "provenance"
        ],
        "type": "object"
      },
      "FieldEvidence": {
        "properties": {
          "field": {
            "$ref": "#/components/schemas/ConfigFieldId"
          },
          "resolution": {
            "$ref": "#/components/schemas/EvidenceValue_FieldResolution"
          },
          "steps": {
            "items": {
              "$ref": "#/components/schemas/ResolutionStep"
            },
            "type": "array"
          },
          "value": {
            "$ref": "#/components/schemas/EvidenceValue_ConfigValue"
          }
        },
        "required": [
          "field",
          "value",
          "resolution",
          "steps"
        ],
        "type": "object"
      },
      "HostAssociationState": {
        "enum": [
          "Registered",
          "Renewed",
          "Released",
          "InvalidatedByReset"
        ],
        "type": "string"
      },
      "HostObservationId": {
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
      "HostObservationKind": {
        "enum": [
          "NativeRoot",
          "ProcessHost"
        ],
        "type": "string"
      },
      "HostObservationRef": {
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "id": {
            "$ref": "#/components/schemas/HostObservationId"
          }
        },
        "required": [
          "daemon",
          "id"
        ],
        "type": "object"
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
      "InboxClaimRef": {
        "properties": {
          "batch": {
            "$ref": "#/components/schemas/BatchId"
          },
          "database_generation": {
            "type": "string"
          },
          "scope": {
            "$ref": "#/components/schemas/ScopeRef"
          },
          "target": {
            "$ref": "#/components/schemas/ActivationTarget"
          }
        },
        "required": [
          "scope",
          "database_generation",
          "target",
          "batch"
        ],
        "type": "object"
      },
      "InspectionReady": {
        "properties": {
          "build_id": {
            "type": "string"
          },
          "build_version": {
            "type": "string"
          },
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "generation": {
            "$ref": "#/components/schemas/Evidence_String"
          },
          "process": {
            "$ref": "#/components/schemas/ObservedProcess"
          },
          "protocol_version": {
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
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "daemon",
          "protocol_version",
          "build_version",
          "build_id",
          "process",
          "generation",
          "sampled_at"
        ],
        "type": "object"
      },
      "InspectionResponse": {
        "oneOf": [
          {
            "properties": {
              "DaemonChanged": {
                "properties": {
                  "actual": {
                    "$ref": "#/components/schemas/DaemonRef"
                  },
                  "expected": {
                    "$ref": "#/components/schemas/DaemonRef"
                  }
                },
                "required": [
                  "expected",
                  "actual"
                ],
                "type": "object"
              }
            },
            "required": [
              "DaemonChanged"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Runtime": {
                "$ref": "#/components/schemas/SerdeResult_RuntimeResponse_RuntimeReadError"
              }
            },
            "required": [
              "Runtime"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Configuration": {
                "$ref": "#/components/schemas/SerdeResult_ConfigurationResponse_ConfigurationError"
              }
            },
            "required": [
              "Configuration"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Events": {
                "$ref": "#/components/schemas/SerdeResult_ObservationResponse_ObservationReadError"
              }
            },
            "required": [
              "Events"
            ],
            "type": "object"
          }
        ]
      },
      "InspectionResult": {
        "properties": {
          "ready": {
            "$ref": "#/components/schemas/InspectionReady"
          },
          "received_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "response": {
            "$ref": "#/components/schemas/InspectionResponse"
          }
        },
        "required": [
          "ready",
          "response",
          "received_at"
        ],
        "type": "object"
      },
      "InvocationCaptureRef": {
        "properties": {
          "invocation": {
            "$ref": "#/components/schemas/ClientInvocationId"
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
          "invocation",
          "revision"
        ],
        "type": "object"
      },
      "LatencyHistogram": {
        "properties": {
          "counts": {
            "items": {
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
            "type": "array"
          }
        },
        "required": [
          "counts"
        ],
        "type": "object"
      },
      "LocalConfigSnapshotRef": {
        "properties": {
          "capture": {
            "$ref": "#/components/schemas/InvocationCaptureRef"
          },
          "sequence": {
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
          "capture",
          "sequence"
        ],
        "type": "object"
      },
      "LocalFailure": {
        "properties": {
          "at": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationTime",
                "description": "None establishes no ordering or age; never substitute a sentinel or a new clock."
              }
            ]
          },
          "kind": {
            "$ref": "#/components/schemas/LocalFailureKind"
          },
          "os_code": {
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
                "x-agentx-native-integer": "int32"
              },
              {
                "type": "null"
              }
            ],
            "x-agentx-native-schema": {
              "format": "int32",
              "type": [
                "integer",
                "null"
              ]
            }
          },
          "stage": {
            "$ref": "#/components/schemas/ClientStage"
          }
        },
        "required": [
          "stage",
          "kind"
        ],
        "type": "object"
      },
      "LocalFailureKind": {
        "enum": [
          "MissingScope",
          "NotDirectory",
          "PermissionDenied",
          "EndpointMissing",
          "ConnectionRefused",
          "TimedOut",
          "Busy",
          "Incompatible",
          "Protocol",
          "Unsupported",
          "PeerRejected",
          "StaleGeneration",
          "OutcomeUnknown",
          "ClockUnavailable",
          "Io"
        ],
        "type": "string"
      },
      "MessageAcceptanceRef": {
        "properties": {
          "database_generation": {
            "type": "string"
          },
          "request_id": {
            "$ref": "#/components/schemas/RequestId"
          },
          "scope": {
            "$ref": "#/components/schemas/ScopeRef"
          }
        },
        "required": [
          "scope",
          "database_generation",
          "request_id"
        ],
        "type": "object"
      },
      "MonotonicMillis": {
        "additionalProperties": false,
        "description": "Whole milliseconds from the origin identified by an accompanying ClockId.",
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
          "description": "Whole milliseconds from the origin identified by an accompanying ClockId.",
          "format": "u-int64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "NotRunReason": {
        "oneOf": [
          {
            "properties": {
              "Admission": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "Admission"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Dispatch": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "Dispatch"
            ],
            "type": "object"
          }
        ]
      },
      "ObservationCursor": {
        "properties": {
          "sequence": {
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
          "stream": {
            "$ref": "#/components/schemas/ObservationStreamId"
          }
        },
        "required": [
          "stream",
          "sequence"
        ],
        "type": "object"
      },
      "ObservationEpoch": {
        "description": "Dataset fence: daemon incarnation plus the exact committed DB generation.",
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "database_generation": {
            "type": "string"
          }
        },
        "required": [
          "daemon",
          "database_generation"
        ],
        "type": "object"
      },
      "ObservationFamily": {
        "enum": [
          "Runtime",
          "ClientReported",
          "Configuration",
          "DomainCommitted"
        ],
        "type": "string"
      },
      "ObservationFilter": {
        "properties": {
          "client_attempt": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ClientAttemptId"
              }
            ]
          },
          "configuration": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ConfigSnapshotRef"
              }
            ]
          },
          "domain_event": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/DurableEventRef"
              }
            ]
          },
          "family": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationFamily"
              }
            ]
          },
          "minimum_severity": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationSeverity"
              }
            ]
          },
          "origin": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/EvidenceOrigin"
              }
            ]
          },
          "participant": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ParticipantRef"
              }
            ]
          },
          "request": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestObservationId"
              }
            ]
          },
          "request_id": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestId"
              }
            ]
          }
        },
        "type": "object"
      },
      "ObservationHead": {
        "properties": {
          "losses": {
            "$ref": "#/components/schemas/CollectionLosses"
          },
          "newest": {
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
          "observed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "oldest": {
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
          "stream": {
            "$ref": "#/components/schemas/ObservationStreamId"
          }
        },
        "required": [
          "stream",
          "oldest",
          "newest",
          "observed_at",
          "losses"
        ],
        "type": "object"
      },
      "ObservationInput": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "payload": {
            "$ref": "#/components/schemas/ObservationPayload"
          },
          "source": {
            "$ref": "#/components/schemas/EvidenceStamp"
          }
        },
        "required": [
          "epoch",
          "source",
          "payload"
        ],
        "type": "object"
      },
      "ObservationPage": {
        "properties": {
          "head": {
            "$ref": "#/components/schemas/ObservationHead"
          },
          "next": {
            "$ref": "#/components/schemas/ObservationWindow"
          },
          "records": {
            "items": {
              "$ref": "#/components/schemas/ObservationRecord"
            },
            "type": "array"
          }
        },
        "required": [
          "records",
          "next",
          "head"
        ],
        "type": "object"
      },
      "ObservationPayload": {
        "oneOf": [
          {
            "properties": {
              "Runtime": {
                "$ref": "#/components/schemas/RuntimeEvent"
              }
            },
            "required": [
              "Runtime"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ClientReported": {
                "$ref": "#/components/schemas/ClientReportObservation"
              }
            },
            "required": [
              "ClientReported"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Configuration": {
                "$ref": "#/components/schemas/ConfigObservationEvent"
              }
            },
            "required": [
              "Configuration"
            ],
            "type": "object"
          },
          {
            "properties": {
              "DomainCommitted": {
                "$ref": "#/components/schemas/DomainCommitObservation"
              }
            },
            "required": [
              "DomainCommitted"
            ],
            "type": "object"
          }
        ]
      },
      "ObservationRead": {
        "properties": {
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
            "x-agentx-native-integer": "u-int16",
            "x-agentx-native-schema": {
              "format": "u-int16",
              "minimum": 0,
              "type": "integer"
            }
          },
          "window": {
            "$ref": "#/components/schemas/ObservationWindow"
          }
        },
        "required": [
          "window",
          "limit"
        ],
        "type": "object"
      },
      "ObservationReadResult": {
        "oneOf": [
          {
            "properties": {
              "Page": {
                "$ref": "#/components/schemas/ObservationPage"
              }
            },
            "required": [
              "Page"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Gap": {
                "properties": {
                  "first_missing": {
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
                  "head": {
                    "$ref": "#/components/schemas/ObservationHead"
                  },
                  "last_missing": {
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
                  "first_missing",
                  "last_missing",
                  "head"
                ],
                "type": "object"
              }
            },
            "required": [
              "Gap"
            ],
            "type": "object"
          },
          {
            "properties": {
              "EpochChanged": {
                "properties": {
                  "head": {
                    "$ref": "#/components/schemas/ObservationHead"
                  },
                  "requested": {
                    "$ref": "#/components/schemas/ObservationEpoch"
                  }
                },
                "required": [
                  "requested",
                  "head"
                ],
                "type": "object"
              }
            },
            "required": [
              "EpochChanged"
            ],
            "type": "object"
          },
          {
            "properties": {
              "StreamChanged": {
                "properties": {
                  "head": {
                    "$ref": "#/components/schemas/ObservationHead"
                  },
                  "requested": {
                    "$ref": "#/components/schemas/ObservationStreamId"
                  }
                },
                "required": [
                  "requested",
                  "head"
                ],
                "type": "object"
              }
            },
            "required": [
              "StreamChanged"
            ],
            "type": "object"
          }
        ]
      },
      "ObservationRecord": {
        "properties": {
          "collected_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "cursor": {
            "$ref": "#/components/schemas/ObservationCursor"
          },
          "input": {
            "$ref": "#/components/schemas/ObservationInput"
          }
        },
        "required": [
          "cursor",
          "collected_at",
          "input"
        ],
        "type": "object"
      },
      "ObservationSeverity": {
        "enum": [
          "Info",
          "Notice",
          "Warning",
          "Error"
        ],
        "type": "string"
      },
      "ObservationStreamId": {
        "properties": {
          "epoch": {
            "$ref": "#/components/schemas/ObservationEpoch"
          },
          "nonce": {
            "type": "string"
          }
        },
        "required": [
          "epoch",
          "nonce"
        ],
        "type": "object"
      },
      "ObservationTime": {
        "description": "A producer's clock-domain time; unavailable wall time remains None.",
        "properties": {
          "clock": {
            "$ref": "#/components/schemas/ClockId"
          },
          "monotonic_ms": {
            "$ref": "#/components/schemas/MonotonicMillis"
          },
          "unix_ms": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/UnixMillis"
              }
            ]
          }
        },
        "required": [
          "clock",
          "monotonic_ms"
        ],
        "type": "object"
      },
      "ObservationWindow": {
        "properties": {
          "after": {
            "$ref": "#/components/schemas/ObservationCursor"
          },
          "filter": {
            "$ref": "#/components/schemas/ObservationFilter"
          },
          "through": {
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
          "after",
          "through",
          "filter"
        ],
        "type": "object"
      },
      "ObservedProcess": {
        "properties": {
          "incarnation": {
            "type": [
              "string",
              "null"
            ]
          },
          "pid": {
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
          "pid"
        ],
        "type": "object"
      },
      "OperationDomain": {
        "enum": [
          "Inspection",
          "Management",
          "Collaboration",
          "Participants",
          "Messages",
          "Hosts",
          "Events",
          "Queries",
          "Maintenance",
          "Relay"
        ],
        "type": "string"
      },
      "OperationLabel": {
        "properties": {
          "domain": {
            "$ref": "#/components/schemas/OperationDomain"
          },
          "variant": {
            "type": "string"
          }
        },
        "required": [
          "domain",
          "variant"
        ],
        "type": "object"
      },
      "OriginRef": {
        "properties": {
          "attempt": {
            "$ref": "#/components/schemas/ClientAttemptId"
          },
          "invocation": {
            "$ref": "#/components/schemas/InvocationCaptureRef"
          }
        },
        "required": [
          "invocation",
          "attempt"
        ],
        "type": "object"
      },
      "OutputEvidenceRef": {
        "properties": {
          "database_generation": {
            "type": "string"
          },
          "observation": {
            "$ref": "#/components/schemas/OutputObservationId"
          },
          "participant": {
            "$ref": "#/components/schemas/ParticipantRef"
          },
          "scope": {
            "$ref": "#/components/schemas/ScopeRef"
          }
        },
        "required": [
          "scope",
          "database_generation",
          "participant",
          "observation"
        ],
        "type": "object"
      },
      "OutputObservationId": {
        "description": "Observation-only UUID. Never derive this from a native output receipt token.",
        "type": "string"
      },
      "ParticipantFieldId": {
        "enum": [
          "Timeout",
          "SubagentTimeout",
          "Hints"
        ],
        "type": "string"
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
      "PeerObservationId": {
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
      "PeerObservationRef": {
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "id": {
            "$ref": "#/components/schemas/PeerObservationId"
          }
        },
        "required": [
          "daemon",
          "id"
        ],
        "type": "object"
      },
      "ProvenanceRelation": {
        "enum": [
          "SameClass",
          "DifferentClass",
          "Unknown"
        ],
        "type": "string"
      },
      "ReadCall": {
        "additionalProperties": false,
        "properties": {
          "expected_daemon": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/DaemonRef"
              }
            ]
          },
          "read": {
            "$ref": "#/components/schemas/BrowserRead"
          }
        },
        "required": [
          "read"
        ],
        "type": "object"
      },
      "ReadConsistency": {
        "enum": [
          "SingleSourceRead",
          "MultipleNonAtomicReads",
          "InputsOnly"
        ],
        "type": "string"
      },
      "ReadInputError": {
        "enum": [
          "LimitOutOfRange",
          "BoundaryOutOfRange",
          "WrongScope",
          "InvalidReference",
          "FilterTooLarge"
        ],
        "type": "string"
      },
      "ReadResult": {
        "oneOf": [
          {
            "properties": {
              "Inspection": {
                "$ref": "#/components/schemas/InspectionResult"
              }
            },
            "required": [
              "Inspection"
            ],
            "type": "object"
          },
          {
            "properties": {
              "LocalFailure": {
                "$ref": "#/components/schemas/LocalFailure"
              }
            },
            "required": [
              "LocalFailure"
            ],
            "type": "object"
          }
        ]
      },
      "RecordDisposition": {
        "oneOf": [
          {
            "properties": {
              "Recorded": {
                "$ref": "#/components/schemas/ObservationCursor"
              }
            },
            "required": [
              "Recorded"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Dropped": {
                "$ref": "#/components/schemas/DropReason"
              }
            },
            "required": [
              "Dropped"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Suppressed"
            ],
            "type": "string"
          }
        ]
      },
      "ReplyObservation": {
        "oneOf": [
          {
            "enum": [
              "Pending"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Observed": {
                "properties": {
                  "wait": {
                    "$ref": "#/components/schemas/ReplyWaitObservation"
                  },
                  "write": {
                    "$ref": "#/components/schemas/ReplyWriteObservation"
                  }
                },
                "required": [
                  "wait",
                  "write"
                ],
                "type": "object"
              }
            },
            "required": [
              "Observed"
            ],
            "type": "object"
          }
        ]
      },
      "ReplyWaitObservation": {
        "oneOf": [
          {
            "enum": [
              "Waiting"
            ],
            "type": "string"
          },
          {
            "properties": {
              "ResponseAvailable": {
                "properties": {
                  "at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "at"
                ],
                "type": "object"
              }
            },
            "required": [
              "ResponseAvailable"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Abandoned": {
                "properties": {
                  "at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  },
                  "code": {
                    "$ref": "#/components/schemas/RuntimeFailureCode"
                  }
                },
                "required": [
                  "at",
                  "code"
                ],
                "type": "object"
              }
            },
            "required": [
              "Abandoned"
            ],
            "type": "object"
          }
        ]
      },
      "ReplyWriteObservation": {
        "oneOf": [
          {
            "enum": [
              "NotAttempted"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Finished": {
                "properties": {
                  "at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  }
                },
                "required": [
                  "at"
                ],
                "type": "object"
              }
            },
            "required": [
              "Finished"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Failed": {
                "properties": {
                  "at": {
                    "$ref": "#/components/schemas/ObservationTime"
                  },
                  "code": {
                    "$ref": "#/components/schemas/RuntimeFailureCode"
                  }
                },
                "required": [
                  "at",
                  "code"
                ],
                "type": "object"
              }
            },
            "required": [
              "Failed"
            ],
            "type": "object"
          }
        ]
      },
      "RequestId": {
        "description": "Stable caller-generated identifier. Reuse after a lost reply only where the\noperation explicitly promises durable deduplication with the same input.",
        "type": "string"
      },
      "RequestObservation": {
        "properties": {
          "caller_deadline_unix_ms": {
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
          "configuration": {
            "$ref": "#/components/schemas/ConfigurationTrace"
          },
          "connection": {
            "$ref": "#/components/schemas/ConnectionRef"
          },
          "decoded_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "dequeued_at": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationTime"
              }
            ]
          },
          "effective_deadline": {
            "$ref": "#/components/schemas/EvidenceValue_ObservationTime"
          },
          "enqueued_at": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationTime"
              }
            ]
          },
          "execution": {
            "$ref": "#/components/schemas/ExecutionObservation"
          },
          "generation_after_return": {
            "$ref": "#/components/schemas/EvidenceValue_String"
          },
          "generation_at_decode": {
            "type": "string"
          },
          "generation_at_dispatch": {
            "type": [
              "string",
              "null"
            ]
          },
          "last_updated_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "operation": {
            "$ref": "#/components/schemas/OperationLabel"
          },
          "origin": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/OriginRef"
              }
            ]
          },
          "peer": {
            "$ref": "#/components/schemas/EvidenceValue_PeerObservationRef"
          },
          "reference": {
            "$ref": "#/components/schemas/RequestObservationRef"
          },
          "reply": {
            "$ref": "#/components/schemas/ReplyObservation"
          },
          "request_id": {
            "$ref": "#/components/schemas/RequestId"
          }
        },
        "required": [
          "reference",
          "connection",
          "peer",
          "request_id",
          "operation",
          "decoded_at",
          "effective_deadline",
          "generation_at_decode",
          "generation_after_return",
          "execution",
          "reply",
          "configuration",
          "last_updated_at"
        ],
        "type": "object"
      },
      "RequestObservationId": {
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
      "RequestObservationRef": {
        "properties": {
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "id": {
            "$ref": "#/components/schemas/RequestObservationId"
          }
        },
        "required": [
          "daemon",
          "id"
        ],
        "type": "object"
      },
      "RequestSelection": {
        "oneOf": [
          {
            "enum": [
              "AllRetained"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Active"
            ],
            "type": "string"
          },
          {
            "properties": {
              "LogicalRequest": {
                "$ref": "#/components/schemas/RequestId"
              }
            },
            "required": [
              "LogicalRequest"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Attempt": {
                "$ref": "#/components/schemas/ClientAttemptId"
              }
            },
            "required": [
              "Attempt"
            ],
            "type": "object"
          }
        ]
      },
      "ResolutionStep": {
        "properties": {
          "disposition": {
            "$ref": "#/components/schemas/CandidateDisposition"
          },
          "source": {
            "$ref": "#/components/schemas/ConfigSourceRef"
          },
          "validation": {
            "$ref": "#/components/schemas/CandidateValidation"
          }
        },
        "required": [
          "source",
          "disposition",
          "validation"
        ],
        "type": "object"
      },
      "RetainedClientReport": {
        "properties": {
          "observation": {
            "$ref": "#/components/schemas/ClientReportObservation"
          },
          "received_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "reference": {
            "$ref": "#/components/schemas/ClientReportRef"
          }
        },
        "required": [
          "reference",
          "observation",
          "received_at"
        ],
        "type": "object"
      },
      "RetentionEnd": {
        "enum": [
          "Age",
          "CountPressure",
          "BytePressure",
          "Reset"
        ],
        "type": "string"
      },
      "RulesRevision": {
        "properties": {
          "disclosure_revision": {
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
          "field_catalog_revision": {
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
          "native_build_id": {
            "type": "string"
          },
          "resolution_revision": {
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
          "native_build_id",
          "field_catalog_revision",
          "resolution_revision",
          "disclosure_revision"
        ],
        "type": "object"
      },
      "RuntimeCorrelation": {
        "properties": {
          "client_attempt": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ClientAttemptId"
              }
            ]
          },
          "connection": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ConnectionId"
              }
            ]
          },
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "epoch": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationEpoch"
              }
            ]
          },
          "request": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestObservationId"
              }
            ]
          },
          "request_id": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/RequestId"
              }
            ]
          }
        },
        "required": [
          "daemon"
        ],
        "type": "object"
      },
      "RuntimeCounters": {
        "properties": {
          "accepted_connections": {
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
          "admission_busy": {
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
          "application_error": {
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
          "application_ok": {
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
          "decoded_calls": {
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
          "dispatch_not_run": {
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
          "dispatch_started": {
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
          "queue_admissions": {
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
          "reply_wait_abandoned": {
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
          "reply_write_failed": {
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
          "reply_write_ok": {
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
          "accepted_connections",
          "decoded_calls",
          "queue_admissions",
          "admission_busy",
          "dispatch_started",
          "dispatch_not_run",
          "application_ok",
          "application_error",
          "reply_write_ok",
          "reply_write_failed",
          "reply_wait_abandoned"
        ],
        "type": "object"
      },
      "RuntimeCoverage": {
        "properties": {
          "full_host_census": {
            "type": "boolean"
          },
          "host_projections_dropped": {
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
          "local_reports_dropped": {
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
          "peer_projections_dropped": {
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
          "request_details_dropped": {
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
          "retained_rows_evicted": {
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
          "request_details_dropped",
          "retained_rows_evicted",
          "peer_projections_dropped",
          "host_projections_dropped",
          "local_reports_dropped",
          "full_host_census"
        ],
        "type": "object"
      },
      "RuntimeEvent": {
        "properties": {
          "at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "correlation": {
            "$ref": "#/components/schemas/RuntimeCorrelation"
          },
          "fact": {
            "$ref": "#/components/schemas/RuntimeFact"
          },
          "source": {
            "$ref": "#/components/schemas/RuntimeProducer"
          }
        },
        "required": [
          "at",
          "source",
          "correlation",
          "fact"
        ],
        "type": "object"
      },
      "RuntimeFact": {
        "oneOf": [
          {
            "properties": {
              "LifecycleChanged": {
                "$ref": "#/components/schemas/DaemonLifecycle"
              }
            },
            "required": [
              "LifecycleChanged"
            ],
            "type": "object"
          },
          {
            "enum": [
              "ConnectionAccepted"
            ],
            "type": "string"
          },
          {
            "enum": [
              "HelloValidated"
            ],
            "type": "string"
          },
          {
            "properties": {
              "ConnectionClosed": {
                "$ref": "#/components/schemas/ConnectionCloseReason"
              }
            },
            "required": [
              "ConnectionClosed"
            ],
            "type": "object"
          },
          {
            "properties": {
              "CallDecoded": {
                "$ref": "#/components/schemas/OperationLabel"
              }
            },
            "required": [
              "CallDecoded"
            ],
            "type": "object"
          },
          {
            "enum": [
              "Queued"
            ],
            "type": "string"
          },
          {
            "properties": {
              "AdmissionRejected": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "AdmissionRejected"
            ],
            "type": "object"
          },
          {
            "enum": [
              "DispatchStarted"
            ],
            "type": "string"
          },
          {
            "properties": {
              "DispatchRejected": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "DispatchRejected"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ApplicationReturned": {
                "$ref": "#/components/schemas/ApplicationResultClass"
              }
            },
            "required": [
              "ApplicationReturned"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ReplyWriteFinished": {
                "properties": {
                  "failure": {
                    "oneOf": [
                      {
                        "type": "null"
                      },
                      {
                        "$ref": "#/components/schemas/RuntimeFailureCode"
                      }
                    ]
                  }
                },
                "type": "object"
              }
            },
            "required": [
              "ReplyWriteFinished"
            ],
            "type": "object"
          },
          {
            "properties": {
              "ReplyWaitAbandoned": {
                "$ref": "#/components/schemas/RuntimeFailureCode"
              }
            },
            "required": [
              "ReplyWaitAbandoned"
            ],
            "type": "object"
          },
          {
            "properties": {
              "HostAssociationObserved": {
                "properties": {
                  "host": {
                    "$ref": "#/components/schemas/HostObservationRef"
                  },
                  "state": {
                    "$ref": "#/components/schemas/HostAssociationState"
                  }
                },
                "required": [
                  "host",
                  "state"
                ],
                "type": "object"
              }
            },
            "required": [
              "HostAssociationObserved"
            ],
            "type": "object"
          }
        ]
      },
      "RuntimeFailureCode": {
        "enum": [
          "Busy",
          "Deadline",
          "Stopping",
          "DispatcherClosed",
          "ScopeOwnershipLost",
          "Incompatible",
          "Protocol",
          "StaleGeneration",
          "GenerationUnavailable",
          "OutcomeUnknown",
          "Unsupported",
          "Io",
          "Internal"
        ],
        "type": "string"
      },
      "RuntimeGauges": {
        "properties": {
          "business_connections": {
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
            "x-agentx-native-integer": "u-int16",
            "x-agentx-native-schema": {
              "format": "u-int16",
              "minimum": 0,
              "type": "integer"
            }
          },
          "executing_jobs": {
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
            "x-agentx-native-integer": "u-int8",
            "x-agentx-native-schema": {
              "format": "u-int8",
              "minimum": 0,
              "type": "integer"
            }
          },
          "inspection_connections": {
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
            "x-agentx-native-integer": "u-int8",
            "x-agentx-native-schema": {
              "format": "u-int8",
              "minimum": 0,
              "type": "integer"
            }
          },
          "queued_jobs": {
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
            "x-agentx-native-integer": "u-int16",
            "x-agentx-native-schema": {
              "format": "u-int16",
              "minimum": 0,
              "type": "integer"
            }
          }
        },
        "required": [
          "business_connections",
          "queued_jobs",
          "executing_jobs",
          "inspection_connections"
        ],
        "type": "object"
      },
      "RuntimeList": {
        "properties": {
          "before": {
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
            "x-agentx-native-integer": "u-int16",
            "x-agentx-native-schema": {
              "format": "u-int16",
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
      "RuntimeMetricWindow": {
        "properties": {
          "application_ms": {
            "$ref": "#/components/schemas/LatencyHistogram"
          },
          "counters": {
            "$ref": "#/components/schemas/RuntimeCounters"
          },
          "epoch": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationEpoch"
              }
            ]
          },
          "from": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "queue_wait_ms": {
            "$ref": "#/components/schemas/LatencyHistogram"
          },
          "server_to_reply_ms": {
            "$ref": "#/components/schemas/LatencyHistogram"
          },
          "through": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "from",
          "through",
          "counters",
          "queue_wait_ms",
          "application_ms",
          "server_to_reply_ms"
        ],
        "type": "object"
      },
      "RuntimePage_ConnectionObservation": {
        "properties": {
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "next_before": {
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
          "rows": {
            "items": {
              "properties": {
                "accepted_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "closed": {
                  "items": false,
                  "prefixItems": [
                    {
                      "description": "A producer's clock-domain time; unavailable wall time remains None.",
                      "properties": {
                        "clock": {
                          "$ref": "#/components/schemas/ClockId"
                        },
                        "monotonic_ms": {
                          "$ref": "#/components/schemas/MonotonicMillis"
                        },
                        "unix_ms": {
                          "oneOf": [
                            {
                              "type": "null"
                            },
                            {
                              "$ref": "#/components/schemas/UnixMillis"
                            }
                          ]
                        }
                      },
                      "required": [
                        "clock",
                        "monotonic_ms"
                      ],
                      "type": "object"
                    },
                    {
                      "enum": [
                        "HelloOnlyEof",
                        "Completed",
                        "ReadFailure",
                        "WriteFailure",
                        "Deadline",
                        "Rejected",
                        "Shutdown"
                      ],
                      "type": "string"
                    }
                  ],
                  "type": [
                    "array",
                    "null"
                  ]
                },
                "origin": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/OriginRef"
                    }
                  ]
                },
                "peer": {
                  "$ref": "#/components/schemas/EvidenceValue_PeerObservationRef"
                },
                "phase": {
                  "$ref": "#/components/schemas/ConnectionPhase"
                },
                "reference": {
                  "$ref": "#/components/schemas/ConnectionRef"
                },
                "request": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/RequestObservationRef"
                    }
                  ]
                }
              },
              "required": [
                "reference",
                "peer",
                "accepted_at",
                "phase"
              ],
              "type": "object"
            },
            "type": "array"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "rows",
          "sampled_at",
          "revision",
          "coverage"
        ],
        "type": "object"
      },
      "RuntimePage_HostObservation": {
        "properties": {
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "next_before": {
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
          "rows": {
            "items": {
              "properties": {
                "child": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/ObservedProcess"
                    }
                  ]
                },
                "epoch": {
                  "$ref": "#/components/schemas/ObservationEpoch"
                },
                "kind": {
                  "$ref": "#/components/schemas/HostObservationKind"
                },
                "last_request": {
                  "$ref": "#/components/schemas/RequestObservationRef"
                },
                "last_validated_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "participant": {
                  "$ref": "#/components/schemas/ParticipantRef"
                },
                "peer": {
                  "$ref": "#/components/schemas/PeerObservationRef"
                },
                "reference": {
                  "$ref": "#/components/schemas/HostObservationRef"
                },
                "state": {
                  "$ref": "#/components/schemas/HostAssociationState"
                },
                "target": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/ActivationTarget"
                    }
                  ]
                }
              },
              "required": [
                "reference",
                "epoch",
                "kind",
                "peer",
                "participant",
                "state",
                "last_validated_at",
                "last_request"
              ],
              "type": "object"
            },
            "type": "array"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "rows",
          "sampled_at",
          "revision",
          "coverage"
        ],
        "type": "object"
      },
      "RuntimePage_PeerObservation": {
        "properties": {
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "next_before": {
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
          "rows": {
            "items": {
              "properties": {
                "first_seen_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "last_seen_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "open_connections": {
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
                  "x-agentx-native-integer": "u-int16",
                  "x-agentx-native-schema": {
                    "format": "u-int16",
                    "minimum": 0,
                    "type": "integer"
                  }
                },
                "process": {
                  "$ref": "#/components/schemas/ObservedProcess"
                },
                "reference": {
                  "$ref": "#/components/schemas/PeerObservationRef"
                }
              },
              "required": [
                "reference",
                "process",
                "first_seen_at",
                "last_seen_at",
                "open_connections"
              ],
              "type": "object"
            },
            "type": "array"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "rows",
          "sampled_at",
          "revision",
          "coverage"
        ],
        "type": "object"
      },
      "RuntimePage_RequestObservation": {
        "properties": {
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "next_before": {
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
          "rows": {
            "items": {
              "properties": {
                "caller_deadline_unix_ms": {
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
                "configuration": {
                  "$ref": "#/components/schemas/ConfigurationTrace"
                },
                "connection": {
                  "$ref": "#/components/schemas/ConnectionRef"
                },
                "decoded_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "dequeued_at": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/ObservationTime"
                    }
                  ]
                },
                "effective_deadline": {
                  "$ref": "#/components/schemas/EvidenceValue_ObservationTime"
                },
                "enqueued_at": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/ObservationTime"
                    }
                  ]
                },
                "execution": {
                  "$ref": "#/components/schemas/ExecutionObservation"
                },
                "generation_after_return": {
                  "$ref": "#/components/schemas/EvidenceValue_String"
                },
                "generation_at_decode": {
                  "type": "string"
                },
                "generation_at_dispatch": {
                  "type": [
                    "string",
                    "null"
                  ]
                },
                "last_updated_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "operation": {
                  "$ref": "#/components/schemas/OperationLabel"
                },
                "origin": {
                  "oneOf": [
                    {
                      "type": "null"
                    },
                    {
                      "$ref": "#/components/schemas/OriginRef"
                    }
                  ]
                },
                "peer": {
                  "$ref": "#/components/schemas/EvidenceValue_PeerObservationRef"
                },
                "reference": {
                  "$ref": "#/components/schemas/RequestObservationRef"
                },
                "reply": {
                  "$ref": "#/components/schemas/ReplyObservation"
                },
                "request_id": {
                  "$ref": "#/components/schemas/RequestId"
                }
              },
              "required": [
                "reference",
                "connection",
                "peer",
                "request_id",
                "operation",
                "decoded_at",
                "effective_deadline",
                "generation_at_decode",
                "generation_after_return",
                "execution",
                "reply",
                "configuration",
                "last_updated_at"
              ],
              "type": "object"
            },
            "type": "array"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "rows",
          "sampled_at",
          "revision",
          "coverage"
        ],
        "type": "object"
      },
      "RuntimePage_RetainedClientReport": {
        "properties": {
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "next_before": {
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
          "rows": {
            "items": {
              "properties": {
                "observation": {
                  "$ref": "#/components/schemas/ClientReportObservation"
                },
                "received_at": {
                  "$ref": "#/components/schemas/ObservationTime"
                },
                "reference": {
                  "$ref": "#/components/schemas/ClientReportRef"
                }
              },
              "required": [
                "reference",
                "observation",
                "received_at"
              ],
              "type": "object"
            },
            "type": "array"
          },
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "rows",
          "sampled_at",
          "revision",
          "coverage"
        ],
        "type": "object"
      },
      "RuntimeProducer": {
        "enum": [
          "Admission",
          "ConnectionWorker",
          "Dispatcher"
        ],
        "type": "string"
      },
      "RuntimeSnapshot": {
        "properties": {
          "activity": {
            "$ref": "#/components/schemas/DispatchActivity"
          },
          "admission_tick_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "coverage": {
            "$ref": "#/components/schemas/RuntimeCoverage"
          },
          "daemon": {
            "$ref": "#/components/schemas/DaemonRef"
          },
          "dispatcher_progress_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "gauges": {
            "$ref": "#/components/schemas/RuntimeGauges"
          },
          "generation": {
            "$ref": "#/components/schemas/Evidence_String"
          },
          "lifecycle": {
            "$ref": "#/components/schemas/DaemonLifecycle"
          },
          "lifecycle_changed_at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "lifetime": {
            "$ref": "#/components/schemas/RuntimeCounters"
          },
          "oldest_queued_at": {
            "oneOf": [
              {
                "type": "null"
              },
              {
                "$ref": "#/components/schemas/ObservationTime"
              }
            ]
          },
          "recent": {
            "$ref": "#/components/schemas/RuntimeMetricWindow"
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
          "sampled_at": {
            "$ref": "#/components/schemas/ObservationTime"
          }
        },
        "required": [
          "daemon",
          "sampled_at",
          "generation",
          "lifecycle",
          "activity",
          "lifecycle_changed_at",
          "admission_tick_at",
          "dispatcher_progress_at",
          "gauges",
          "lifetime",
          "recent",
          "coverage",
          "revision"
        ],
        "type": "object"
      },
      "ScopeRef": {
        "description": "Canonical local data scope, supplied by an existing resolver or lease.",
        "properties": {
          "canonical_scope": {
            "type": "string"
          }
        },
        "required": [
          "canonical_scope"
        ],
        "type": "object"
      },
      "SerdeResult_ConfigurationResponse_ConfigurationError": {
        "description": "serde serializes Result<T, E> as the externally tagged Ok/Err union.\nUse only as a ToSchema value_type override; never convert runtime results to it.",
        "oneOf": [
          {
            "properties": {
              "Ok": {
                "oneOf": [
                  {
                    "properties": {
                      "Head": {
                        "$ref": "#/components/schemas/ConfigurationHead"
                      }
                    },
                    "required": [
                      "Head"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Snapshot": {
                        "$ref": "#/components/schemas/ConfigurationRead_SnapshotLookup"
                      }
                    },
                    "required": [
                      "Snapshot"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Comparison": {
                        "$ref": "#/components/schemas/ConfigurationRead_CompareConfigResult"
                      }
                    },
                    "required": [
                      "Comparison"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Reported": {
                        "$ref": "#/components/schemas/ConfigSnapshotRef"
                      }
                    },
                    "required": [
                      "Reported"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Ok"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Err": {
                "oneOf": [
                  {
                    "properties": {
                      "Query": {
                        "$ref": "#/components/schemas/ConfigurationQueryError"
                      }
                    },
                    "required": [
                      "Query"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Report": {
                        "$ref": "#/components/schemas/ConfigReportError"
                      }
                    },
                    "required": [
                      "Report"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "DaemonChanged": {
                        "properties": {
                          "current": {
                            "$ref": "#/components/schemas/DaemonRef"
                          },
                          "expected": {
                            "$ref": "#/components/schemas/DaemonRef"
                          }
                        },
                        "required": [
                          "expected",
                          "current"
                        ],
                        "type": "object"
                      }
                    },
                    "required": [
                      "DaemonChanged"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Err"
            ],
            "type": "object"
          }
        ]
      },
      "SerdeResult_ObservationResponse_ObservationReadError": {
        "description": "serde serializes Result<T, E> as the externally tagged Ok/Err union.\nUse only as a ToSchema value_type override; never convert runtime results to it.",
        "oneOf": [
          {
            "properties": {
              "Ok": {
                "oneOf": [
                  {
                    "properties": {
                      "Head": {
                        "$ref": "#/components/schemas/ObservationHead"
                      }
                    },
                    "required": [
                      "Head"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Read": {
                        "$ref": "#/components/schemas/ObservationReadResult"
                      }
                    },
                    "required": [
                      "Read"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Ok"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Err": {
                "oneOf": [
                  {
                    "properties": {
                      "InvalidInput": {
                        "$ref": "#/components/schemas/ReadInputError"
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
                    "properties": {
                      "GenerationUnavailable": {
                        "properties": {
                          "daemon": {
                            "$ref": "#/components/schemas/DaemonRef"
                          }
                        },
                        "required": [
                          "daemon"
                        ],
                        "type": "object"
                      }
                    },
                    "required": [
                      "GenerationUnavailable"
                    ],
                    "type": "object"
                  },
                  {
                    "enum": [
                      "Busy"
                    ],
                    "type": "string"
                  },
                  {
                    "enum": [
                      "Unavailable"
                    ],
                    "type": "string"
                  }
                ]
              }
            },
            "required": [
              "Err"
            ],
            "type": "object"
          }
        ]
      },
      "SerdeResult_RuntimeResponse_RuntimeReadError": {
        "description": "serde serializes Result<T, E> as the externally tagged Ok/Err union.\nUse only as a ToSchema value_type override; never convert runtime results to it.",
        "oneOf": [
          {
            "properties": {
              "Ok": {
                "oneOf": [
                  {
                    "properties": {
                      "Snapshot": {
                        "$ref": "#/components/schemas/RuntimeSnapshot"
                      }
                    },
                    "required": [
                      "Snapshot"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Requests": {
                        "$ref": "#/components/schemas/RuntimePage_RequestObservation"
                      }
                    },
                    "required": [
                      "Requests"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "RequestDetail": {
                        "$ref": "#/components/schemas/RequestObservation"
                      }
                    },
                    "required": [
                      "RequestDetail"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Connections": {
                        "$ref": "#/components/schemas/RuntimePage_ConnectionObservation"
                      }
                    },
                    "required": [
                      "Connections"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Peers": {
                        "$ref": "#/components/schemas/RuntimePage_PeerObservation"
                      }
                    },
                    "required": [
                      "Peers"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "Hosts": {
                        "$ref": "#/components/schemas/RuntimePage_HostObservation"
                      }
                    },
                    "required": [
                      "Hosts"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "ClientReport": {
                        "$ref": "#/components/schemas/ClientReportReceipt"
                      }
                    },
                    "required": [
                      "ClientReport"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "ClientReports": {
                        "$ref": "#/components/schemas/RuntimePage_RetainedClientReport"
                      }
                    },
                    "required": [
                      "ClientReports"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "ClientReportDetail": {
                        "$ref": "#/components/schemas/RetainedClientReport"
                      }
                    },
                    "required": [
                      "ClientReportDetail"
                    ],
                    "type": "object"
                  }
                ]
              }
            },
            "required": [
              "Ok"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Err": {
                "oneOf": [
                  {
                    "enum": [
                      "InvalidInput"
                    ],
                    "type": "string"
                  },
                  {
                    "enum": [
                      "ScopeMismatch"
                    ],
                    "type": "string"
                  },
                  {
                    "properties": {
                      "DaemonChanged": {
                        "properties": {
                          "actual": {
                            "$ref": "#/components/schemas/DaemonRef"
                          }
                        },
                        "required": [
                          "actual"
                        ],
                        "type": "object"
                      }
                    },
                    "required": [
                      "DaemonChanged"
                    ],
                    "type": "object"
                  },
                  {
                    "properties": {
                      "DetailUnavailable": {
                        "$ref": "#/components/schemas/UnknownReason"
                      }
                    },
                    "required": [
                      "DetailUnavailable"
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
                      "Busy"
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
                    "enum": [
                      "Unsupported"
                    ],
                    "type": "string"
                  }
                ]
              }
            },
            "required": [
              "Err"
            ],
            "type": "object"
          }
        ]
      },
      "ServiceFailureClass": {
        "enum": [
          "InvalidInput",
          "IdentityRequired",
          "Forbidden",
          "NotFound",
          "Conflict",
          "StaleFence",
          "Unavailable",
          "Unsupported",
          "OutcomeUnknown",
          "Internal"
        ],
        "type": "string"
      },
      "SnapshotLookup": {
        "oneOf": [
          {
            "properties": {
              "Found": {
                "$ref": "#/components/schemas/ConfigSnapshot"
              }
            },
            "required": [
              "Found"
            ],
            "type": "object"
          },
          {
            "properties": {
              "Missing": {
                "properties": {
                  "reason": {
                    "$ref": "#/components/schemas/ConfigMissing"
                  },
                  "reference": {
                    "$ref": "#/components/schemas/ConfigSnapshotRef"
                  }
                },
                "required": [
                  "reference",
                  "reason"
                ],
                "type": "object"
              }
            },
            "required": [
              "Missing"
            ],
            "type": "object"
          }
        ]
      },
      "SourceRead": {
        "properties": {
          "at": {
            "$ref": "#/components/schemas/ObservationTime"
          },
          "id": {
            "$ref": "#/components/schemas/SourceReadId"
          },
          "read_revision": {
            "type": "string"
          },
          "resource": {
            "$ref": "#/components/schemas/SourceResource"
          },
          "state": {
            "$ref": "#/components/schemas/SourceReadState"
          }
        },
        "required": [
          "id",
          "resource",
          "read_revision",
          "at",
          "state"
        ],
        "type": "object"
      },
      "SourceReadId": {
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
        "x-agentx-native-integer": "u-int16",
        "x-agentx-native-schema": {
          "format": "u-int16",
          "minimum": 0,
          "type": "integer"
        }
      },
      "SourceReadState": {
        "oneOf": [
          {
            "enum": [
              "Read"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Missing"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Unreadable"
            ],
            "type": "string"
          },
          {
            "enum": [
              "SyntaxInvalid"
            ],
            "type": "string"
          },
          {
            "properties": {
              "NotObserved": {
                "$ref": "#/components/schemas/UnknownReason"
              }
            },
            "required": [
              "NotObserved"
            ],
            "type": "object"
          }
        ]
      },
      "SourceResource": {
        "enum": [
          "ConfigToml",
          "LaunchEnvironmentFile",
          "TerminalPresets",
          "PersistentDevRoot"
        ],
        "type": "string"
      },
      "UnixMillis": {
        "additionalProperties": false,
        "description": "Whole milliseconds since the Unix epoch; chronology, not causal ordering.",
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
          "description": "Whole milliseconds since the Unix epoch; chronology, not causal ordering.",
          "format": "u-int64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "UnknownReason": {
        "description": "A bounded observation cannot establish this value for the stated reason.",
        "enum": [
          "NotCaptured",
          "NotSupported",
          "ObservationFailed",
          "Expired",
          "Evicted",
          "Pressure",
          "GenerationUnavailable"
        ],
        "type": "string"
      },
      "ValueRelation": {
        "oneOf": [
          {
            "enum": [
              "Equal"
            ],
            "type": "string"
          },
          {
            "enum": [
              "Different"
            ],
            "type": "string"
          },
          {
            "properties": {
              "Unknown": {
                "$ref": "#/components/schemas/ComparisonUnknown"
              }
            },
            "required": [
              "Unknown"
            ],
            "type": "object"
          }
        ]
      }
    }
  },
  "info": {
    "contact": {
      "name": "agentx"
    },
    "description": "Passive fixed existing scope. Every integer uses the native $integer marker codec. Exact numeric local Host and same Origin admission; POST application/json only. BrowserRead is the closed thirteen-read boundary. LocalFailure remains a 200 observation outcome, distinct from HTTP errors; an unavailable daemon is never acquired or started. Preserve daemon/epoch/stream references, stale/gap and clock comparability. No report, mutation, arbitrary native request or scope-selection body is exposed.",
    "license": {
      "identifier": "MIT",
      "name": "MIT"
    },
    "title": "AgentX Observation",
    "version": "1"
  },
  "openapi": "3.1.0",
  "paths": {
    "/api/v1/read": {
      "post": {
        "operationId": "observationRead",
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
                  "$ref": "#/components/schemas/ReadResult"
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
        "summary": "Execute one of thirteen passive reads; preserve LocalFailure and native refusal.",
        "tags": [
          "super"
        ]
      }
    },
    "/api/v1/scope": {
      "post": {
        "operationId": "observationScope",
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
                  "$ref": "#/components/schemas/ScopeRef"
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
        "summary": "Return the fixed existing scope without daemon acquisition or configuration.",
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
