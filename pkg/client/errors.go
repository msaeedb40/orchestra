package client

import "fmt"

// APIError represents an error response from the Orchestra API server.
type APIError struct {
	StatusCode int
	Message    string
	Body       string
}

// Error implements the error interface.
func (e *APIError) Error() string {
	return fmt.Sprintf("orchestra API error %d: %s", e.StatusCode, e.Message)
}

// IsNotFound returns true when err is an *APIError with StatusCode 404.
func IsNotFound(err error) bool {
	if err == nil {
		return false
	}
	if apiErr, ok := err.(*APIError); ok {
		return apiErr.StatusCode == 404
	}
	return false
}

// IsConflict returns true when err is an *APIError with StatusCode 409.
func IsConflict(err error) bool {
	if err == nil {
		return false
	}
	if apiErr, ok := err.(*APIError); ok {
		return apiErr.StatusCode == 409
	}
	return false
}
