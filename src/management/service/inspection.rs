//! Public Inspector caller binding. Collection/execution remain internal runtime work.
use super::*;
use crate::inspection::contract as inspect;

impl ManagementService {
    pub(crate) fn inspection_snapshot(&self, input: GenerationInput) -> ServiceResult<inspect::Snapshot> {
        match self.inspection_call(&input.generation, inspect::Request::Read { generation: input.generation.clone() })? {
            inspect::Response::Snapshot(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection snapshot response".into())),
        }
    }

    pub(crate) fn configure_inspection(&self, input: inspect::ConfigureInput) -> ServiceResult<inspect::Snapshot> {
        let generation = input.generation.clone();
        match self.inspection_call(&generation, inspect::Request::Configure(input))? {
            inspect::Response::Snapshot(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection configuration response".into())),
        }
    }

    pub(crate) fn analyze_inspection(&self, input: inspect::AnalyzeInput) -> ServiceResult<inspect::AnalysisJob> {
        let generation = input.generation.clone();
        match self.inspection_call(&generation, inspect::Request::Analyze(input))? {
            inspect::Response::Job(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection analysis response".into())),
        }
    }

    pub(crate) fn cancel_inspection_analysis(&self, input: inspect::CancelAnalysisInput) -> ServiceResult<inspect::AnalysisJob> {
        let generation = input.generation.clone();
        match self.inspection_call(&generation, inspect::Request::CancelAnalysis(input))? {
            inspect::Response::Job(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection cancellation response".into())),
        }
    }

    pub(crate) fn request_continuation(&self, input: inspect::RequestContinuationInput) -> ServiceResult<inspect::RecoveryAttempt> {
        let generation = input.generation.clone();
        match self.inspection_call(&generation, inspect::Request::RequestContinuation(input))? {
            inspect::Response::Attempt(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection continuation response".into())),
        }
    }

    fn inspection_call(&self, generation: &str, input: inspect::Request) -> ServiceResult<inspect::Response> {
        let client = self.client(generation)?;
        match client.call(crate::service::Request::Inspection(input))? {
            crate::service::Response::Inspection(value) => Ok(value),
            _ => Err(ServiceError::Internal("unexpected inspection response".into())),
        }
    }
}
